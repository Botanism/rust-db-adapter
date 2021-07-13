// `db_session`, and `teardown_db` is based on https://github.com/almetica/almetica/blob/9d9688d3d1ddddae2594ed18fe78ac6b5718d1e7/src/model.rs#L375
// licensed under AGPL 3.0 by almetica

pub mod guild_test_info {
    use serenity::model::id::{ChannelId, GuildId, RoleId};
    //beware the types are do not exactly represent those expected by the end user of lib.
    //they are such because of const restrictions and because it doesn't affect test quality
    pub const FIRST_ID: GuildId = GuildId(5844);
    pub const FIRST_WELCOME_MESSAGE: Option<&str> = Some("hello");
    pub const FIRST_GOODBYE_MESSAGE: Option<String> = None;
    pub const FIRST_ADVERTISE: bool = true;
    pub const FIRST_ADMIN_CHAN: Option<ChannelId> = Some(ChannelId(87904));
    pub const FIRST_POLL_CHANS: [ChannelId; 3] =
        [ChannelId(2323), ChannelId(664), ChannelId(1212054)];
    pub const FIRST_PRIV_MANAGER: [RoleId; 3] = [RoleId(22522), RoleId(44943544), RoleId(4444444)];
    pub const FIRST_PRIV_ADMIN: [RoleId; 2] = [RoleId(22522), RoleId(44943544)];
    pub const FIRST_PRIV_EVENT: [RoleId; 1] = [RoleId(48201365)];
    pub const SECOND_ID: GuildId = GuildId(8750);
    pub const SECOND_WELCOME_MESSAGE: Option<String> = None;
    pub const SECOND_GOODBYE_MESSAGE: Option<&str> = Some("goodbye");
    pub const SECOND_ADVERTISE: bool = false;
    pub const SECOND_ADMIN_CHAN: Option<ChannelId> = None;
    pub const SECOND_POLL_CHANS: [ChannelId; 3] =
        [ChannelId(5406), ChannelId(254102), ChannelId(5455)];
    pub const SECOND_PRIV_MANAGER: [RoleId; 3] = [RoleId(843934), RoleId(3504), RoleId(084304)];
    pub const SECOND_PRIV_ADMIN: [RoleId; 2] = [RoleId(843934), RoleId(3504)];
    pub const SECOND_PRIV_EVENT: [RoleId; 1] = [RoleId(984762)];
}

// WE don't use the `query!` macro because it only looks up the `DATABASE_URL` env var
// when tests should rather use `TEST_DB_URL`
#[macro_use]
pub mod db_test_interface {
    use std::borrow::Cow;
    use std::env;
    use std::panic;

    use dotenv::dotenv;
    use paste::paste;
    use rand::{thread_rng, Rng};
    use sqlx::{migrate, Connection, PgConnection, Result};
    use tokio::runtime::Runtime;

    pub fn db_session<F>(test: F) -> Result<()>
    where
        //we only pass the url instead of the connection because creating it requires async
        F: FnOnce(&str, Runtime) -> Result<()> + panic::UnwindSafe,
    {
        dotenv().ok();
        let base_url = env::var("TEST_DB_URL").expect("TEST_DB_URL was not set");

        // TODO: ugly! try to find a better way to do the following -> #[tokio::test] does the same
        let this_runtime = Runtime::new().unwrap();
        let db_name = this_runtime.block_on(async { db_setup(&base_url).await })?;

        let result = panic::catch_unwind(|| {
            let db_url = format!("{}/{}", base_url, db_name);
            //TODO: find a way to already execute `test` in an async closure (unwind bound complains)
            if let Err(e) = test(&db_url, Runtime::new().unwrap()) {
                panic!("Error occured while executing test: {:?}", e)
            }
        });

        this_runtime.block_on(async { teardown_db(&base_url, &db_name).await })?;

        assert!(result.is_ok());

        Ok(())
    }

    //we create a db to only for one test
    async fn db_setup(base_url: &str) -> Result<String> {
        //not a truly random name but chances are slim that two identical names will be generated
        let mut db_name = String::from("botanist_test_");
        let random_id: u128 = thread_rng().gen();
        db_name.push_str(random_id.to_string().as_str());

        let mut default_conn = PgConnection::connect(&base_url).await?;
        // TODO: investigave why using the `query!` macro would not compile
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            //Executor is only impl for &mut Connection
            .execute(&mut default_conn)
            .await?;

        //we don't want to continue on the default DB
        let db_url = format!("{}/{}", base_url, db_name);
        let mut new_conn = PgConnection::connect(&db_url).await?;
        apply_migrations(&mut new_conn).await?;
        insert_dummy(new_conn).await?;

        Ok(db_name)
    }

    ///we drop the db after testing
    async fn teardown_db(base_url: &str, name: &str) -> Result<()> {
        let mut conn = PgConnection::connect(&base_url).await?;

        sqlx::query(&format!("DROP DATABASE {};", name))
            .execute(&mut conn)
            .await?;

        // Drop all other connections to the database -> is this really necessary?
        sqlx::query(
            format!(
                r#"SELECT pg_terminate_backend(pg_stat_activity.pid)
                           FROM pg_stat_activity
                           WHERE datname = '{}'
                           AND pid <> pg_backend_pid();"#,
                name
            )
            .as_ref(),
        )
        .execute(&mut conn)
        .await?;
        conn.close().await?;

        Ok(())
    }

    /// we apply all the migrations from `migration` to our test DB
    async fn apply_migrations(conn: &mut PgConnection) -> Result<()> {
        migrate!("./migrations").run(conn).await?;
        Ok(())
    }

    // TODO: find how to return a literal instead of a String
    macro_rules! prepare_guild_row {
        ($row:literal) => {{
            use super::guild_test_info::*;
            format!("INSERT INTO guilds(id, welcome_message, goodbye_message, advertise, admin_chan, poll_chans, priv_admin, priv_manager, priv_event) VALUES ({}, {}, {}, {}, {}, array[{}, {}, {}], array[{}, {}], array[{}, {}, {}], array[{}])",
            paste! {[<$row _ID>]},
            paste!{stringify_option([<$row _WELCOME_MESSAGE>])},
            paste!{stringify_option([<$row _GOODBYE_MESSAGE>])},
            paste!{[<$row _ADVERTISE>]},
            paste!{stringify_option([<$row _ADMIN_CHAN>])},
            paste!{[<$row _POLL_CHANS>][0]},
            paste!{[<$row _POLL_CHANS>][1]},
            paste!{[<$row _POLL_CHANS>][2]},
            paste!{[<$row _PRIV_ADMIN>][0]},
            paste!{[<$row _PRIV_ADMIN>][1]},
            paste!{[<$row _PRIV_MANAGER>][0]},
            paste!{[<$row _PRIV_MANAGER>][1]},
            paste!{[<$row _PRIV_MANAGER>][2]},
            paste!{[<$row _PRIV_EVENT>][0]}
        )
        }};
    }

    /// inserts some dummy values into the dabase to allow tests to be relevant
    async fn insert_dummy(mut conn: PgConnection) -> Result<()> {
        //guild mock data
        sqlx::query(&prepare_guild_row!("FIRST"))
            .execute(&mut conn)
            .await?;
        sqlx::query(&prepare_guild_row!("SECOND"))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    fn stringify_option<'a, T: ToString + std::fmt::Display>(option: Option<T>) -> Cow<'a, str> {
        match option {
            Some(value) => Cow::Owned(format!("'{}'", value)),
            None => Cow::Borrowed("NULL"),
        }
    }

    #[macro_export]
    macro_rules! db_test {
        (async fn $name:ident $($tt:tt)*) => {
            #[test]
            fn $name() -> Result<()> {
                async fn inner $($tt)*
                db_session(|db_url, runtime| {
                    runtime.block_on(async {
                        let pool = sqlx::PgPool::connect(&db_url).await?;
                        inner(pool).await
                    })
                })
            }
        }
    }

    #[allow(unused_imports)]
    pub(crate) use db_test;
}
