//! A [Guild]'s preferences (aka: configuration)
//!
//! Botanist is shipped with many features, some of which
//! work out of the box (ex: `ping`). Other on the other hand
//! rely on guild-specific datum. This data as a whole is called
//! the [Guild]'s preferences or configuration.
//! It notably includes the privilege (see [`Privilege`]) system
//! but also conviniences such as welcome messages, administration
//! channels or advertisement policy.
//!
//! [Guild]: serenity::model::guild::Guild

use crate::{as_pg_array, from_i64, stringify_option, to_i64};
use async_recursion::async_recursion;
use serenity::model::id::{ChannelId, GuildId, RoleId};
use sqlx::{query, Executor, Postgres, Row};
use thiserror::Error;

enum MessageType {
    Welcome,
    Goodbye,
}

impl AsRef<str> for MessageType {
    fn as_ref(&self) -> &str {
        match self {
            MessageType::Welcome => "welcome_message",
            MessageType::Goodbye => "goodbye_message",
        }
    }
}

/// Errors originating from the `GuildConfig` wrapper
#[derive(Error, Debug)]
pub enum GuildConfigError {
    #[error("`{field:?}` can't be over 2000 chracters")]
    MessageTooLong { field: String },
    #[error("could not execute query")]
    SqlxError(#[from] sqlx::Error),
    #[error("{role:?} doesn't have privilege {privilege:?}")]
    RoleNoPrivilege { role: RoleId, privilege: Privilege },
    #[error("GuildId({0}) already has a configuration entry")]
    AlreadyExists(GuildId),
}

type Result<Return> = std::result::Result<Return, GuildConfigError>;

/// Wraps around a `guilds` row
///
/// [`GuildConfig`] provides an API covering every common use-case. When it doesn't piecing methods
/// together is simple and safe. As much as possible it tries to issue as little queries as possible
/// so you can confidently use the methods.
///
/// # Connection (`conn`) parameter.
///
/// For the sake of flexibility as little constraints as possible were put on the methods.
/// The major example of this is the `conn` parameter which generally accepts anything that
/// implements: [`sqlx::Executor`]. This means both [`sqlx::PgPool`] and
/// [`sqlx::PgConnection`] can be used. However some methods need to issue multiple queries.
/// As such it requires a `conn` that implements [`Copy`]. In those cases simply pass a `&PgPool`
///
/// # Errors
///
/// For simplicty's sake only methods that can give other errors than [`sqlx::Error`] have a section
/// detailing the error.
///
/// All methods provided by [`Self`] return a `Result` which's [`Err`] variant is
/// [`GuildConfigError`]. One of the later's variant wraps around [`sqlx::Error`] which is returned by
/// every [`sqlx`] method that interacts with the database. These are all about database errors, which for the
/// user of the library, should only be caused by incorrect setup (see [`crate`]).
#[derive(Debug)]
pub struct GuildConfig(GuildId);

impl From<GuildId> for GuildConfig {
    fn from(src: GuildId) -> GuildConfig {
        GuildConfig(src)
    }
}

impl GuildConfig {
    /// Adds a new entry to the `guilds` table.
    ///
    /// # Errors
    ///
    /// Errors with [`GuildConfigError::AlreadyExists`] if a row with the same `id` already exists in the DB
    pub async fn new<'a, 'b, PgExec: Executor<'a, Database = Postgres> + Copy>(
        conn: PgExec,
        builder: GuildConfigBuilder<'b>,
    ) -> Result<Self> {
        let guild_config = GuildConfig::from(builder.id);
        if guild_config.exists(conn).await? {
            return Err(GuildConfigError::AlreadyExists(builder.id));
        };

        let poll_chans = builder
            .poll_chans
            .map(|vec| vec.iter().map(|int| to_i64(int.0)).collect::<Vec<i64>>());
        query!(
            "INSERT INTO guilds(id, welcome_message, goodbye_message, advertise, admin_chan, poll_chans, priv_admin, priv_manager, priv_event) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            to_i64(builder.id),
            builder.welcome_message,
            builder.goodbye_message,
            builder.advertise,
            builder.admin_chan.map(|int| to_i64(int.0)),
            poll_chans.as_deref(),
            &builder.priv_admin.iter().map(|role| to_i64(role.0)).collect::<Vec<i64>>(),
            &builder.priv_manager.iter().map(|role| to_i64(role.0)).collect::<Vec<i64>>(),
            &builder.priv_event.iter().map(|role| to_i64(role.0)).collect::<Vec<i64>>(),
        )
        .execute(conn)
        .await?;

        Ok(guild_config)
    }

    /// `true` if the guild exists in the database, `false` otherwise.
    pub async fn exists<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<bool> {
        let this_id: i64 = to_i64(self.0);
        let ids = query!("SELECT id FROM guilds").fetch_all(conn).await?;
        Ok(ids.iter().any(|record| record.id == this_id))
    }

    async fn get_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        msg_ty: MessageType,
    ) -> Result<Option<String>> {
        Ok(sqlx::query(&format!(
            "SELECT {} FROM guilds WHERE id={}",
            msg_ty.as_ref(),
            to_i64(self.0),
        ))
        .fetch_one(conn)
        .await?
        .try_get(msg_ty.as_ref())?)
    }

    /// `welcome_message` currently in use
    ///
    /// This is the message sent to new users when they join. Disabled if [`None`].
    pub async fn get_welcome_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<Option<String>> {
        self.get_message(conn, MessageType::Welcome).await
    }

    /// `goodbye_message` currently in use
    pub async fn get_goodbye_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<Option<String>> {
        self.get_message(conn, MessageType::Goodbye).await
    }

    async fn set_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        msg_ty: MessageType,
        msg: Option<&str>,
    ) -> Result<()> {
        if let Some(string) = msg {
            if string.len() > 2000 {
                return Err(GuildConfigError::MessageTooLong {
                    field: msg_ty.as_ref().to_string(),
                });
            }
        }
        sqlx::query(&format!(
            "UPDATE guilds SET {}={} WHERE id={}",
            msg_ty.as_ref(),
            stringify_option(msg),
            to_i64(self.0)
        ))
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Change `welcome_message`
    ///
    /// # Error
    /// If the message is over discord's length limit for a message (2000 characters) the query will not be made
    /// and the method will return [`GuildConfigError::MessageTooLong`].
    pub async fn set_welcome_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        msg: Option<&str>,
    ) -> Result<()> {
        self.set_message(conn, MessageType::Welcome, msg).await
    }

    /// Change `goodbye_message`
    ///
    /// # Error
    /// If the message is over discord's length limit for a message (2000 characters) the query will not be made
    /// and the method will return [`GuildConfigError::MessageTooLong`].
    pub async fn set_goodbye_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        msg: Option<&str>,
    ) -> Result<()> {
        self.set_message(conn, MessageType::Goodbye, msg).await
    }

    /// `advertise`
    pub async fn get_advertise<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<bool> {
        Ok(
            query!("SELECT advertise FROM guilds WHERE id=$1", to_i64(self.0))
                .fetch_one(conn)
                .await?
                .advertise,
        )
    }

    /// Change the advertisement policy
    pub async fn set_advertise<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        policy: bool,
    ) -> Result<()> {
        query!(
            "UPDATE guilds SET advertise=$1 WHERE id=$2",
            policy,
            to_i64(self.0)
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// `admin_chan`
    ///
    /// Events demanding the attention of guild admins are posted to the admin channel.
    /// This includes but is not limited to slap notices, upcoming updates, etc.
    pub async fn get_admin_chan<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<Option<ChannelId>> {
        Ok(
            query!("SELECT admin_chan FROM guilds WHERE id=$1", to_i64(self.0))
                .fetch_one(conn)
                // maybe use fetch_optional? It works like this though :shrug:
                .await?
                .admin_chan
                .map(|id| from_i64(id)),
        )
    }

    /// Change the `admin_chan`
    pub async fn set_admin_chan<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        chan: Option<ChannelId>,
    ) -> Result<()> {
        query!(
            "UPDATE guilds SET admin_chan=$1 WHERE id=$2",
            match chan {
                Some(chan) => Some(to_i64(chan.0)),
                None => None,
            },
            to_i64(self.0)
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn get_raw_roles_with<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        privilege: Privilege,
    ) -> Result<Vec<i64>> {
        Ok(sqlx::query(&format!(
            "SELECT {} FROM guilds WHERE id={}",
            privilege.as_ref(),
            to_i64(self.0)
        ))
        .fetch_one(conn)
        .await?
        .try_get(privilege.as_ref())?)
    }

    /// Roles with the specified privilege
    pub async fn get_roles_with<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
        privilege: Privilege,
    ) -> Result<Vec<RoleId>> {
        Ok(self
            .get_raw_roles_with(conn, privilege)
            .await?
            .iter()
            .map(|int| from_i64(*int))
            .collect())
    }

    async fn update_privilege<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        ids: &[i64],
        privilege: Privilege,
    ) -> Result<()> {
        sqlx::query(&format!(
            "UPDATE guilds SET {}={} WHERE id={}",
            privilege.as_ref(),
            as_pg_array(ids),
            to_i64(self.0)
        ))
        .execute(conn)
        .await?;
        Ok(())
    }

    //WARN: the Copy bound implies only immutable references can be passed
    async fn grant_single_privilege<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        id: RoleId,
        privilege: Privilege,
    ) -> Result<()> {
        let role_id = i64::from(id);
        let mut roles = self.get_raw_roles_with(conn, privilege).await?;
        roles.push(role_id);
        self.update_privilege(conn, &roles, privilege).await
    }

    /// Gives a role a privilege
    #[async_recursion] // because `async fn` doesn't support recursion
    pub async fn grant_privilege<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        id: RoleId,
        privilege: Privilege,
    ) -> Result<()> {
        match privilege {
            Privilege::Admin => {
                self.grant_single_privilege(conn, id, Privilege::Manager)
                    .await?;
            }
            Privilege::Manager | Privilege::Event => (),
        };
        self.grant_single_privilege(conn, id, privilege).await
    }

    /// Strips a role from a privilege
    // TODO: Consider using pg's `array_remove` utility instead, see: https://popsql.com/learn-sql/postgresql/how-to-modify-arrays-in-postgresql
    #[async_recursion] // because `async fn` doesn't support recursion
    pub async fn deny_privilege<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        id: RoleId,
        privilege: Privilege,
    ) -> Result<()> {
        let to_remove = i64::from(id);
        match privilege {
            Privilege::Admin => self.deny_privilege(conn, id, Privilege::Manager).await?,
            Privilege::Manager | Privilege::Event => (),
        }
        let mut roles = self.get_raw_roles_with(conn, privilege).await?;
        let index = roles.iter().position(|int| *int == to_remove).ok_or(
            GuildConfigError::RoleNoPrivilege {
                role: id,
                privilege,
            },
        )?;
        roles.swap_remove(index);
        self.update_privilege(conn, &roles, privilege).await?;

        Ok(())
    }

    /// If all roles have a privilege
    pub async fn have_privilege<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        roles: &[RoleId],
        privilege: Privilege,
    ) -> Result<bool> {
        let ids = roles.iter().map(|role| i64::from(*role));

        let db_roles = self.get_raw_roles_with(conn, privilege).await?;
        for id in ids {
            if !db_roles.contains(&id) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// If a role has a privilege
    pub async fn has_privilege<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        role: RoleId,
        privilege: Privilege,
    ) -> Result<bool> {
        let id = i64::from(role);
        Ok(self
            .get_raw_roles_with(conn, privilege)
            .await?
            .contains(&id))
    }
    // TODO: make a get_raw_privileges to make less queries when possible

    /// Id a role has *all* specified privileges
    pub async fn has_privileges<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        role: RoleId,
        privileges: &[Privilege],
    ) -> Result<bool> {
        let privs = self.get_privileges_for(conn, role).await?;
        for privilege in privileges {
            if !privs.contains(privilege) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// All privileges granted to a role
    pub async fn get_privileges_for<'a, PgExec: Executor<'a, Database = Postgres> + Copy>(
        &self,
        conn: PgExec,
        role: RoleId,
    ) -> Result<Vec<Privilege>> {
        let mut privs = Vec::with_capacity(3);
        if self.has_privilege(conn, role, Privilege::Admin).await? {
            privs.push(Privilege::Admin);
            privs.push(Privilege::Manager);
        } else if self.has_privilege(conn, role, Privilege::Manager).await? {
            privs.push(Privilege::Manager);
        }
        if self.has_privilege(conn, role, Privilege::Event).await? {
            privs.push(Privilege::Event);
        }
        Ok(privs)
    }
}

/// Bot's permission system
///
/// Botanist handles permissions through a different system than Discord. This way server admins
/// can fine tune permissions so that users who should not have access to some discord permissions
/// can still fully use the bot, or the other way around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    /// The manager privilege provides low-level administration powers such as message deletion (`clear` command).
    ///  Generally it is good for moderators who are tasked with maintaining order.
    Manager,
    /// The admin privilege provides lets one use all but a few features of the bot.
    /// This excludes those that come with [`Privilege::Event`] or those reserved to the bot owner and server owner.
    Admin,
    /// Lets one organise events within the server using the bot's toolset.
    Event,
}

impl AsRef<str> for Privilege {
    fn as_ref(&self) -> &str {
        match self {
            Privilege::Admin => "priv_admin",
            Privilege::Manager => "priv_manager",
            Privilege::Event => "priv_event",
        }
    }
}

/// Builder for new configuration entries
///
/// This should only be used when the bot joins a new [Guild].
/// This builder is used to quickly whip up a new configuration with sensible defaults
/// that can be easilly overriden. For how to use see [`GuildConfig::new()`] and the tests.
///
/// [Guild]: serenity::model::guild::Guild`
#[derive(Debug)]
pub struct GuildConfigBuilder<'a> {
    id: GuildId,
    welcome_message: Option<&'a str>,
    goodbye_message: Option<&'a str>,
    advertise: bool,
    admin_chan: Option<ChannelId>,
    poll_chans: Option<Vec<ChannelId>>,
    priv_manager: Vec<RoleId>,
    priv_admin: Vec<RoleId>,
    priv_event: Vec<RoleId>,
}

impl<'a> GuildConfigBuilder<'a> {
    pub fn new(id: GuildId) -> GuildConfigBuilder<'a> {
        GuildConfigBuilder {
            id,
            welcome_message: None,
            goodbye_message: None,
            advertise: true,
            admin_chan: None,
            poll_chans: None,
            priv_manager: vec![],
            priv_admin: vec![],
            priv_event: vec![],
        }
    }

    pub fn welcome_message(&mut self, msg: &'a str) -> Result<&mut Self> {
        if msg.len() > 2000 {
            Err(GuildConfigError::MessageTooLong {
                field: "welcome_message".into(),
            })
        } else {
            self.welcome_message = Some(msg);
            Ok(self)
        }
    }

    pub fn goodbye_message(&mut self, msg: &'a str) -> Result<&mut Self> {
        if msg.len() > 2000 {
            Err(GuildConfigError::MessageTooLong {
                field: "goodbye_message".into(),
            })
        } else {
            self.goodbye_message = Some(msg);
            Ok(self)
        }
    }

    pub fn advertise(&mut self, v: bool) -> &mut Self {
        self.advertise = v;
        self
    }
}
