//! Interface for the slap system

use crate::stringify_option;
use serenity::{
    futures::TryStreamExt,
    model::id::{GuildId, MessageId, UserId},
};
use sqlx::{query, query_scalar, Executor, Postgres};
use std::convert::TryFrom;
use thiserror::Error;
use tokio_stream::{Stream, StreamExt};

/// Slap-related errors
#[derive(Debug, Error)]
pub enum SlapError {
    #[error("could not execute query")]
    SqlxError(#[from] sqlx::Error),
}

/// Method through which the slap was issued
///
/// Botanist allows slaps to be given either by a member with the
/// `manager` privilege or by a public vote.
//internally uses 0_u64 as Community
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Enforcer {
    /// The verdict was issued by popular vote
    Community,
    /// A manager issed a slap. Their [`UserId`] is encapsulated.
    Manager(UserId),
}

type Result<Return> = std::result::Result<Return, SlapError>;

fn option_to_enforcer(option: Option<i64>) -> Enforcer {
    match option {
        Some(int) => Enforcer::Manager(u64::try_from(int).unwrap().into()),
        None => Enforcer::Community,
    }
}

pub fn enforcer_to_option(enforcer: Enforcer) -> Option<UserId> {
    match enforcer {
        Enforcer::Manager(user) => Some(user),
        Enforcer::Community => None,
    }
}
/// A single slap object
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
            i64::try_from(sentence).unwrap()
        )
        .fetch_optional(conn)
        .await?
        .map(|record| SlapReport {
            sentence,
            offender: UserId(u64::try_from(record.offender).unwrap()),
            enforcer: option_to_enforcer(record.enforcer),
            reason: record.reason,
        }))
    }
}

async fn insert_raw_slap<'a, PgExec: Executor<'a, Database = Postgres>>(
    conn: PgExec,
    sentence: i64,
    guild: i64,
    offender: i64,
    enforcer: Enforcer,
    reason: Option<String>,
) -> Result<()> {
    sqlx::query(&format!("INSERT INTO slaps(sentence, guild, offender, enforcer, reason) VALUES ({}, {}, {}, {}, {})",sentence, guild, offender, stringify_option(enforcer_to_option(enforcer)), stringify_option(reason))).execute(conn).await?;
    Ok(())
}

/// Record of slaps of a guild member
#[derive(Debug, PartialEq, Eq)]
pub struct MemberSlapRecord(GuildId, UserId);

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
            i64::try_from(sentence).unwrap(),
            i64::try_from(self.0).unwrap(),
            i64::try_from(self.1).unwrap(),
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
        let offender = i64::try_from(self.1).unwrap();
        query!(
            "SELECT sentence, enforcer, reason FROM slaps WHERE guild=$1 AND offender=$2",
            i64::try_from(self.0).unwrap(),
            offender
        )
        .fetch(conn)
        .map_err(|e| SlapError::from(e))
        .map(move |res| {
            res.map(|record| SlapReport {
                sentence: MessageId(u64::try_from(record.sentence).unwrap()),
                offender: self.1,
                enforcer: match record.enforcer {
                    Some(user) => Enforcer::Manager(UserId(u64::try_from(user).unwrap())),
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
            i64::try_from(self.0).unwrap(),
            i64::try_from(self.1).unwrap(),
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

#[derive(Debug, PartialEq, Eq)]
pub struct GuildSlapRecord(GuildId);

impl GuildSlapRecord {
    ///Adds a slap to the guild
    pub async fn new_slap<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        sentence: MessageId,
        offender: UserId,
        enforcer: Enforcer,
        reason: Option<String>,
    ) -> Result<SlapReport> {
        insert_raw_slap(
            conn,
            i64::try_from(sentence).unwrap(),
            i64::try_from(self.0).unwrap(),
            i64::try_from(offender).unwrap(),
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
            i64::try_from(self.0).unwrap(),
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
            i64::try_from(self.0).unwrap(),
        )
        .fetch(conn)
        .map_err(|e| SlapError::from(e))
        .map(move |res| {
            res.map(|record| SlapReport {
                sentence: MessageId(u64::try_from(record.sentence).unwrap()),
                offender: UserId(u64::try_from(record.offender).unwrap()),
                enforcer: match record.enforcer {
                    Some(user) => Enforcer::Manager(UserId(u64::try_from(user).unwrap())),
                    None => Enforcer::Community,
                },
                reason: record.reason,
            })
        })
    }

    ///A stream over all members with a slap record
    pub fn members<'a, PgExec: Executor<'a, Database = Postgres> + 'a>(
        &'a self,
        conn: PgExec,
    ) -> impl Stream<Item = Result<MemberSlapRecord>> + 'a {
        query!(
            "SELECT DISTINCT offender FROM slaps WHERE guild=$1",
            i64::try_from(self.0).unwrap()
        )
        .fetch(conn)
        .map_err(|e| SlapError::from(e))
        .map(move |res| {
            res.map(|record| {
                MemberSlapRecord(self.0, UserId(u64::try_from(record.offender).unwrap()))
            })
        })
    }
}

impl From<GuildId> for GuildSlapRecord {
    fn from(src: GuildId) -> Self {
        GuildSlapRecord(src)
    }
}
