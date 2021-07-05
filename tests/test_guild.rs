mod framework;
use db_adapter::guild::GuildConfig;
use framework::{db_test_interface::db_session, guild_test_info::*};
use sqlx::{query, Connection, PgConnection, Result};

#[test]
fn test_get_welcome_message() -> Result<()> {
    db_session(|db_url, runtime| {
        runtime.block_on(async {
            let mut conn = PgConnection::connect(&db_url).await?;
            assert_eq!(
                GuildConfig::from(FIRST_ID)
                    .get_welcome_message(&mut conn)
                    .await
                    .unwrap()
                    .unwrap()
                    .as_str(),
                FIRST_WELCOME_MESSAGE.unwrap()
            );
            Ok(())
        })
    })
}
