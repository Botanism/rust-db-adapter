use serenity::model::{
    guild::Role,
    id::{ChannelId, GuildId, RoleId},
};
use sqlx::{query, query_as, Executor, PgPool, Postgres};
use std::convert::TryFrom;
use thiserror::Error;

/// Errors originating from the GuildConfig wrapper
#[derive(Error, Debug)]
pub enum GuildConfigError {
    #[error("`{field:?}` can't be over 2000 chracters")]
    MessageTooLong { field: String },
    #[error("could not execute query")]
    SqlxError(#[from] sqlx::Error),
}

type Result<Return> = std::result::Result<Return, GuildConfigError>;

/// Wraps around a `guilds` row
///
/// Most methods returning a [`std::result::Result`] do so only because the query to the DB may fail
/// If another reason may cause it to fail, it will be documented
#[derive(Debug)]
pub struct GuildConfig(GuildId);

impl From<&GuildConfig> for i64 {
    fn from(src: &GuildConfig) -> i64 {
        i64::try_from(src.0 .0).unwrap()
    }
}

impl From<GuildConfig> for i64 {
    fn from(src: GuildConfig) -> i64 {
        i64::from(&src)
    }
}

impl From<i64> for GuildConfig {
    fn from(src: i64) -> GuildConfig {
        GuildConfig(GuildId(u64::try_from(src).unwrap()))
    }
}

impl GuildConfig {
    /// Adds a new entry to the `guilds` table.
    ///
    /// # Errors
    ///
    /// Errors if a row with the same `id` already exists in the DB
    pub async fn new<'a, PgExec: Executor<'a, Database = Postgres>>(
        conn: PgExec,
        builder: GuildConfigBuilder,
    ) -> Result<Self> {
        todo!()
    }

    /// `welcome_message` currently in use
    ///
    /// This is the message sent to new users when they join. Disabled if [`None`].
    pub async fn get_welcome_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<Option<String>> {
        Ok(
            match query!(
                "SELECT welcome_message FROM guilds where guilds.id= $1",
                i64::from(self)
            )
            .fetch_optional(conn)
            .await?
            {
                Some(s) => s.welcome_message,
                None => None,
            },
        )
    }

    /// Change `welcome_message`
    ///
    /// # Error
    /// If the message is over discord's length limit for a message (2000 characters) the query will not be made
    /// and the method will return [`GuildConfigError::MessageTooLong`].
    pub async fn set_welcome_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
        msg: Option<&str>,
    ) -> Result<()> {
        todo!()
    }

    /// `goodbye_message` currently in use
    pub async fn get_goodbye_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
    ) -> Result<Option<String>> {
        todo!()
    }

    /// Change `goodbye_message`
    ///
    /// # Error
    /// If the message is over discord's length limit for a message (2000 characters) the query will not be made
    /// and the method will return [`GuildConfigError::MessageTooLong`].
    pub async fn set_goodbye_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
        msg: Option<&str>,
    ) -> Result<()> {
        todo!()
    }

    /// `advertise`
    pub async fn get_advertise<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
    ) -> Result<bool> {
        todo!()
    }

    /// Change the advertisement policy
    pub async fn set_advertise<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
        policy: bool,
    ) -> Result<()> {
        todo!()
    }

    /// `admin_chan`
    ///
    /// Events demanding the attention of guild admins are posted to the admin channel.
    /// This includes but is not limited to slap notices, upcoming updates, etc.
    pub async fn get_admin_chan<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
    ) -> Result<Option<ChannelId>> {
        todo!()
    }

    /// Change the `admin_chan`
    pub async fn set_admin_chan<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
        chan: Option<ChannelId>,
    ) -> Result<()> {
        todo!()
    }

    /// Roles with the specified privilege
    pub async fn get_roles_with<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
        privilege: Privilege,
    ) -> Result<Option<Vec<RoleId>>> {
        todo!()
    }

    /// Gives a role a privilege
    pub async fn grant_privilege<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: &PgPool,
        id: RoleId,
        privilege: Privilege,
    ) -> Result<()> {
        todo!()
    }

    /// Strips a role from a privilege
    pub async fn deny_privilege(
        &self,
        conn: &PgPool,
        id: RoleId,
        privilege: Privilege,
    ) -> Result<()> {
        todo!()
    }

    /// All privileges granted to a role
    pub async fn get_privileges(&self, conn: &PgPool, role: RoleId) -> Result<Vec<RoleId>> {
        todo!()
    }
}

/// Bot's permission system
///
/// Botanist handles permissions through a different system than Discord. This way server admins
/// can fine tune permissions so that users who should not have access to some discord permissions
/// can still fully use the bot, or the other way around.
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

#[derive(Debug)]
pub struct GuildConfigBuilder {
    id: GuildId,
    welcome_message: Option<String>,
    goodbye_message: Option<String>,
    advertise: bool,
    admin_chan: Option<ChannelId>,
    poll_chans: Option<Vec<ChannelId>>,
    priv_manager: Vec<RoleId>,
    priv_admin: Vec<RoleId>,
    priv_event: Vec<RoleId>,
}

impl GuildConfigBuilder {
    pub fn new(id: GuildId) -> GuildConfigBuilder {
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

    pub fn welcome_message(&mut self, msg: String) -> Result<&mut Self> {
        if msg.len() > 2000 {
            Err(GuildConfigError::MessageTooLong {
                field: "welcome_message".into(),
            })
        } else {
            self.welcome_message = Some(msg);
            Ok(self)
        }
    }

    pub fn goodbye_message(&mut self, msg: String) -> Result<&mut Self> {
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
