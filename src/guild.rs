use crate::as_pg_array;
use async_recursion::async_recursion;
use serenity::model::id::{ChannelId, GuildId, RoleId};
use sqlx::{query, Executor, Postgres, Row};
use std::convert::TryFrom;
use thiserror::Error;

/// Errors originating from the GuildConfig wrapper
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
/// Most methods returning a [`std::result::Result`] do so only because the query to the DB may fail
/// If another reason may cause it to fail, it will be documented
#[derive(Debug)]
pub struct GuildConfig(GuildId);

impl From<GuildId> for GuildConfig {
    fn from(src: GuildId) -> GuildConfig {
        GuildConfig(src)
    }
}

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
    pub async fn new<'a, 'b, PgExec: Executor<'a, Database = Postgres> + Copy>(
        conn: PgExec,
        builder: GuildConfigBuilder<'b>,
    ) -> Result<Self> {
        let guild_config = GuildConfig::from(builder.id);
        if guild_config.exists(conn).await? {
            return Err(GuildConfigError::AlreadyExists(builder.id));
        };

        let poll_chans = builder.poll_chans.map(|vec| {
            vec.iter()
                .map(|int| i64::try_from(int.0).unwrap())
                .collect::<Vec<i64>>()
        });
        query!(
            "INSERT INTO guilds(id, welcome_message, goodbye_message, advertise, admin_chan, poll_chans, priv_admin, priv_manager, priv_event) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            i64::try_from(builder.id).unwrap(),
            builder.welcome_message,
            builder.goodbye_message,
            builder.advertise,
            builder.admin_chan.map(|int| i64::try_from(int.0).unwrap()),
            poll_chans.as_deref(),
            &builder.priv_admin.iter().map(|role| i64::try_from(role.0).unwrap()).collect::<Vec<i64>>(),
            &builder.priv_manager.iter().map(|role| i64::try_from(role.0).unwrap()).collect::<Vec<i64>>(),
            &builder.priv_event.iter().map(|role| i64::try_from(role.0).unwrap()).collect::<Vec<i64>>(),
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
        let this_id: i64 = self.into();
        let ids = query!("SELECT id FROM guilds").fetch_all(conn).await?;
        return match ids.iter().find(|record| record.id == this_id) {
            Some(_) => Ok(true),
            None => Ok(false),
        };
    }

    // TODO: try and refactor *_*_message into the same underlying methods
    /// `welcome_message` currently in use
    ///
    /// This is the message sent to new users when they join. Disabled if [`None`].
    pub async fn get_welcome_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<Option<String>> {
        Ok(
            match query!(
                "SELECT welcome_message FROM guilds WHERE id= $1",
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
        conn: PgExec,
        msg: Option<&str>,
    ) -> Result<()> {
        query!(
            "UPDATE guilds SET welcome_message=$1 WHERE id=$2",
            msg,
            i64::from(self)
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// `goodbye_message` currently in use
    pub async fn get_goodbye_message<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<Option<String>> {
        Ok(
            match query!(
                "SELECT goodbye_message FROM guilds WHERE id= $1",
                i64::from(self)
            )
            .fetch_optional(conn)
            .await?
            {
                Some(s) => s.goodbye_message,
                None => None,
            },
        )
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
        query!(
            "UPDATE guilds SET goodbye_message=$1 WHERE id=$2",
            msg,
            i64::from(self)
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// `advertise`
    pub async fn get_advertise<'a, PgExec: Executor<'a, Database = Postgres>>(
        &self,
        conn: PgExec,
    ) -> Result<bool> {
        Ok(
            query!("SELECT advertise FROM guilds WHERE id=$1", i64::from(self))
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
            i64::from(self)
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
            query!("SELECT admin_chan FROM guilds WHERE id=$1", i64::from(self))
                .fetch_one(conn)
                // maybe use fetch_optional? It works like this though :shrug:
                .await?
                .admin_chan
                .map(|id| u64::try_from(id).unwrap().into()),
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
                Some(chan) => Some(i64::try_from(u64::from(chan)).unwrap()),
                None => None,
            },
            i64::from(self)
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
            privilege.to_string(),
            i64::from(self)
        ))
        .fetch_one(conn)
        .await?
        .try_get(privilege.to_string().as_str())?)
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
            .map(|int| RoleId(u64::try_from(*int).unwrap()))
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
            privilege.to_string(),
            as_pg_array(ids),
            i64::from(self)
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
        let role_id: i64 = id.into();
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
        let to_remove = i64::try_from(id.0).unwrap();
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
        let ids = roles.iter().map(|role| i64::try_from(role.0).unwrap());

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
        let id = i64::try_from(role.0).unwrap();
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

impl ToString for Privilege {
    fn to_string(&self) -> String {
        match self {
            Privilege::Admin => String::from("priv_admin"),
            Privilege::Manager => String::from("priv_manager"),
            Privilege::Event => String::from("priv_event"),
        }
    }
}

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
