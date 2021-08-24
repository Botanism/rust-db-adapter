//! Interface for the slap system
//!
//! Botanist includes a moderation system called `Slapping`. It serves the purpose of giving warnings
//! to members and keeping a record of all of these warnings in a simple way.
//!
//! ## Errors
//! All methods of this module which return a `Result` do so because sql querries through to the database may
//! fail. As such you should handle [`AdapterError::SqlxError`]. Because it is part of the signature of most methods
//! errors are undocumented if they only return a database error. Otherwise an *Error* section is provided.

use crate::{from_i64, stringify_option, to_i64, AdapterError};
#[cfg(feature = "net")]
use serde::{Deserialize, Serialize};

use serenity::{
    futures::TryStreamExt,
    model::id::{GuildId, MessageId, UserId},
};
use sqlx::{query, query_scalar, Executor, Postgres};
use tokio_stream::{Stream, StreamExt};

/// Method through which the slap was issued
///
/// Botanist allows slaps to be given either by a member with the
/// `manager` privilege or by a public vote.
//internally uses None as Community
#[cfg_attr(feature = "net", derive(Deserialize, Serialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Enforcer {
    /// The verdict was issued by popular vote
    Community,
    /// A manager issued a slap. Their [`UserId`] is encapsulated.
    Manager(UserId),
}

impl From<Option<u64>> for Enforcer {
    fn from(option: Option<u64>) -> Self {
        match option {
            Some(id) => Enforcer::Manager(id.into()),
            None => Enforcer::Community,
        }
    }
}

fn option_to_enforcer(option: Option<i64>) -> Enforcer {
    match option {
        Some(int) => Enforcer::Manager(from_i64(int)),
        None => Enforcer::Community,
    }
}

pub(crate) fn enforcer_to_option(enforcer: Enforcer) -> Option<UserId> {
    match enforcer {
        Enforcer::Manager(user) => Some(user),
        Enforcer::Community => None,
    }
}

type Result<R> = std::result::Result<R, AdapterError>;
/// A single slap object
#[cfg_attr(feature = "net", derive(Deserialize, Serialize))]
#[derive(Debug, PartialEq, Eq)]
pub struct SlapReport {
    /// Message from which the slap originates
    ///
    /// Depending on `enforcer`, `sentence` has a different meaning. If it is [`Enforcer::Community`], then `sentence`
    /// points to the message attributed to the reason of the slap. That is to say the one users collectively reacted
    /// with the slap emoji.
    /// Otherwise it points to the message that issued the slap (so a command message).
    pub sentence: MessageId,
    /// The slapped user.
    pub offender: UserId,
    /// Who delivered the slap.
    ///
    /// See [`Enforcer`] for more information.
    pub enforcer: Enforcer,
    /// The reason for the slap.
    ///
    /// This is [`None`] if `enforcer` is  [`Enforcer::Community`] or if the default reason was used.
    /// The default reason is used when the enforcer doesn't provide a `reason` argument when issueing the slap.
    pub reason: Option<String>,
}

impl SlapReport {
    /// Retrieves a SlapReport
    ///
    /// Returns [`None`] if no such slap exists.
    pub async fn get<'a, PgExec: Executor<'a, Database = Postgres>>(
        conn: PgExec,
        sentence: MessageId,
    ) -> Result<Option<SlapReport>> {
        Ok(query!(
            "SELECT offender, enforcer, reason FROM slaps WHERE sentence=$1",
            to_i64(sentence)
        )
        .fetch_optional(conn)
        .await?
        .map(|record| SlapReport {
            sentence,
            offender: UserId(from_i64(record.offender)),
            enforcer: option_to_enforcer(record.enforcer),
            reason: record.reason,
        }))
    }
}

async fn insert_raw_slap<'a, PgExec: Executor<'a, Database = Postgres>, S: std::fmt::Display>(
    conn: PgExec,
    sentence: i64,
    guild: i64,
    offender: i64,
    enforcer: Enforcer,
    reason: Option<S>,
) -> Result<()> {
    sqlx::query(&format!("INSERT INTO slaps(sentence, guild, offender, enforcer, reason) VALUES ({}, {}, {}, {}, {})",sentence, guild, offender, stringify_option(enforcer_to_option(enforcer)), stringify_option(reason))).execute(conn).await?;
    Ok(())
}

/// Record of slaps of a guild member
#[derive(Debug, PartialEq, Eq)]
pub struct MemberSlapRecord(pub GuildId, pub UserId);

impl MemberSlapRecord {
    ///Adds a slap entry for this member
    pub async fn new_slap<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        sentence: MessageId,
        enforcer: Enforcer,
        reason: Option<String>,
    ) -> Result<SlapReport> {
        insert_raw_slap(
            conn,
            to_i64(sentence),
            to_i64(self.0),
            to_i64(self.1),
            enforcer.clone(),
            //try and remove this clone. Consider making stringify_option more generic for that
            reason.clone(),
        )
        .await?;
        Ok(SlapReport {
            sentence,
            offender: self.1,
            enforcer,
            reason,
        })
    }

    ///A stream over all of the member's slaps
    pub fn slaps<'a, PgExec: Executor<'a, Database = Postgres> + 'a>(
        &'a self,
        conn: PgExec,
    ) -> impl Stream<Item = Result<SlapReport>> + 'a {
        let offender = to_i64(self.1);
        query!(
            "SELECT sentence, enforcer, reason FROM slaps WHERE guild=$1 AND offender=$2",
            to_i64(self.0),
            offender
        )
        .fetch(conn)
        .map_err(|e| AdapterError::from(e))
        .map(move |res| {
            res.map(|record| SlapReport {
                sentence: MessageId(from_i64(record.sentence)),
                offender: self.1,
                enforcer: match record.enforcer {
                    Some(user) => Enforcer::Manager(UserId(from_i64(user))),
                    None => Enforcer::Community,
                },
                reason: record.reason,
            })
        })
    }

    ///The number of slaps of the member
    pub async fn len<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<usize> {
        Ok(query_scalar!(
            r#"SELECT COUNT(sentence) as "count!" FROM slaps WHERE guild=$1 AND offender=$2"#,
            to_i64(self.0),
            to_i64(self.1),
        )
        .fetch_one(conn)
        .await? as usize)
    }
}

impl From<(GuildId, UserId)> for MemberSlapRecord {
    fn from(src: (GuildId, UserId)) -> Self {
        MemberSlapRecord(src.0, src.1)
    }
}

impl From<(GuildSlapRecord, UserId)> for MemberSlapRecord {
    fn from(src: (GuildSlapRecord, UserId)) -> Self {
        MemberSlapRecord(src.0 .0, src.1)
    }
}

/// Record of slaps of a guild
#[derive(Debug, PartialEq, Eq)]
pub struct GuildSlapRecord(pub GuildId);

impl GuildSlapRecord {
    ///Adds a slap to the guild
    pub async fn new_slap<
        'a,
        PgExec: Executor<'a, Database = Postgres> + Copy,
        S: std::fmt::Display,
    >(
        &self,
        conn: PgExec,
        sentence: MessageId,
        offender: UserId,
        enforcer: Enforcer,
        reason: Option<S>,
    ) -> Result<SlapReport> {
        let reason = reason.map(|s| s.to_string());
        insert_raw_slap(
            conn,
            to_i64(sentence),
            to_i64(self.0),
            to_i64(offender),
            enforcer.clone(),
            reason.clone(),
        )
        .await?;
        Ok(SlapReport {
            sentence,
            offender,
            enforcer,
            reason,
        })
    }

    ///Number of slaps in the guild
    pub async fn len<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<usize> {
        Ok(query_scalar!(
            // "count!" is to force non-null -> see sqlx::query! docs
            r#"SELECT COUNT(sentence) as "count!" FROM slaps WHERE guild=$1"#,
            to_i64(self.0),
        )
        .fetch_one(conn)
        .await? as usize)
    }

    ///A stream over all slaps of the guild
    pub fn slaps<'a, PgExec: Executor<'a, Database = Postgres> + 'a>(
        &'a self,
        conn: PgExec,
    ) -> impl Stream<Item = Result<SlapReport>> + 'a {
        query!(
            "SELECT sentence, offender, enforcer, reason FROM slaps WHERE guild=$1",
            to_i64(self.0),
        )
        .fetch(conn)
        .map_err(|e| AdapterError::from(e))
        .map(move |res| {
            res.map(|record| SlapReport {
                sentence: MessageId(from_i64(record.sentence)),
                offender: UserId(from_i64(record.offender)),
                enforcer: match record.enforcer {
                    Some(user) => Enforcer::Manager(UserId(from_i64(user))),
                    None => Enforcer::Community,
                },
                reason: record.reason,
            })
        })
    }

    ///A stream over all members with a slap record
    pub fn offenders<'a, PgExec: Executor<'a, Database = Postgres> + 'a>(
        &'a self,
        conn: PgExec,
    ) -> impl Stream<Item = Result<MemberSlapRecord>> + 'a {
        query!(
            "SELECT DISTINCT offender FROM slaps WHERE guild=$1",
            to_i64(self.0)
        )
        .fetch(conn)
        .map_err(|e| AdapterError::from(e))
        .map(move |res| {
            res.map(|record| MemberSlapRecord(self.0, UserId(from_i64(record.offender))))
        })
    }

    ///Number of offending members in the guild
    pub async fn offender_count<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<usize> {
        Ok(query_scalar!(
            // "count!" is to force non-null -> see sqlx::query! docs
            r#"SELECT COUNT(DISTINCT offender) as "count!" FROM slaps WHERE guild=$1"#,
            to_i64(self.0),
        )
        .fetch_one(conn)
        .await? as usize)
    }
}

impl From<GuildId> for GuildSlapRecord {
    fn from(src: GuildId) -> Self {
        GuildSlapRecord(src)
    }
}
