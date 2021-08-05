use super::framework::{
    db_test_interface::{db_session, db_test},
    guild_test_info::*,
};
use crate::guild::{GuildConfig, GuildConfigBuilder, GuildConfigError, Privilege};
use macro_rules_attribute::apply;
use serenity::model::id::{ChannelId, GuildId, RoleId};
use sqlx::{PgPool, Result};

#[apply(db_test!)]
async fn test_new(pool: PgPool) -> Result<()> {
    let id = 123456789.into();

    let mut builder = GuildConfigBuilder::new(id);
    let welcome = "Hello dear people";
    let goodbye = "So long my friend";
    builder
        .welcome_message(welcome)
        .unwrap()
        .goodbye_message(goodbye)
        .unwrap();

    let guild_config = GuildConfig::new(&pool, builder).await.unwrap();
    assert!(dbg!(guild_config.exists(&pool).await).unwrap());
    assert_eq!(
        guild_config
            .get_welcome_message(&pool)
            .await
            .unwrap()
            .unwrap()
            .as_str(),
        welcome
    );
    assert_eq!(
        guild_config
            .get_goodbye_message(&pool)
            .await
            .unwrap()
            .unwrap()
            .as_str(),
        goodbye
    );
    assert_eq!(guild_config.get_admin_chan(&pool).await.unwrap(), None);
    assert!(guild_config.get_advertise(&pool).await.unwrap());

    Ok(())
}

#[apply(db_test!)]
async fn test_exists(pool: PgPool) -> Result<()> {
    assert!(GuildConfig::from(FIRST_ID).exists(&pool).await.unwrap());
    assert!(!GuildConfig::from(GuildId(572634589))
        .exists(&pool)
        .await
        .unwrap());
    Ok(())
}

#[apply(db_test!)]
async fn test_some_get_welcome_message(pool: PgPool) -> Result<()> {
    assert_eq!(
        GuildConfig::from(FIRST_ID)
            .get_welcome_message(&pool)
            .await
            .unwrap()
            .unwrap()
            .as_str(),
        FIRST_WELCOME_MESSAGE.unwrap()
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_none_get_welcome_message(pool: PgPool) -> Result<()> {
    assert_eq!(
        GuildConfig::from(SECOND_ID)
            .get_welcome_message(&pool)
            .await
            .unwrap(),
        SECOND_WELCOME_MESSAGE
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_set_welcome_message(pool: PgPool) -> Result<()> {
    let g_config = GuildConfig::from(FIRST_ID);
    g_config
        .set_welcome_message(&pool, Some("welcome message"))
        .await
        .unwrap();
    assert_eq!(
        g_config.get_welcome_message(&pool).await.unwrap(),
        Some("welcome message".to_string())
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_too_long_set_welcome_message(pool: PgPool) -> Result<()> {
    let g_config = GuildConfig::from(FIRST_ID);
    return match g_config.set_welcome_message(&pool, Some(TOO_LONG)).await {
        Err(GuildConfigError::MessageTooLong { field: _ }) => Ok(()),
        _ => panic!(),
    };
}

#[apply(db_test!)]
async fn test_get_advertise(pool: PgPool) -> Result<()> {
    assert_eq!(
        GuildConfig::from(FIRST_ID)
            .get_advertise(&pool)
            .await
            .unwrap(),
        FIRST_ADVERTISE
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_set_advertise(pool: PgPool) -> Result<()> {
    let g_config = GuildConfig::from(FIRST_ID);
    g_config.set_advertise(&pool, false).await.unwrap();
    assert_eq!(g_config.get_advertise(&pool).await.unwrap(), false);
    Ok(())
}

#[apply(db_test!)]
async fn test_some_get_admin_chan(pool: PgPool) -> Result<()> {
    assert_eq!(
        GuildConfig::from(FIRST_ID)
            .get_admin_chan(&pool)
            .await
            .unwrap()
            .unwrap(),
        FIRST_ADMIN_CHAN.unwrap()
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_none_get_admin_chan(pool: PgPool) -> Result<()> {
    assert_eq!(
        GuildConfig::from(SECOND_ID)
            .get_admin_chan(&pool)
            .await
            .unwrap(),
        SECOND_ADMIN_CHAN
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_set_admin_chan(pool: PgPool) -> Result<()> {
    let g_config = GuildConfig::from(FIRST_ID);
    g_config
        .set_admin_chan(&pool, Some(ChannelId(1234567890)))
        .await
        .unwrap();
    assert_eq!(
        g_config.get_admin_chan(&pool).await.unwrap(),
        Some(ChannelId(1234567890))
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_get_roles_with(pool: PgPool) -> Result<()> {
    let g_config = GuildConfig::from(FIRST_ID);
    assert_eq!(
        g_config
            .get_roles_with(&pool, Privilege::Admin)
            .await
            .unwrap(),
        FIRST_PRIV_ADMIN
    );
    Ok(())
}

#[apply(db_test!)]
async fn test_grant_admin_privilege(pool: PgPool) -> Result<()> {
    let guild_conf = GuildConfig::from(FIRST_ID);
    let role = RoleId(1234567);
    guild_conf
        .grant_privilege(&pool, role, Privilege::Admin)
        .await
        .unwrap();
    //admin priv was given
    //manager priv was also given (invariant check)
    assert!(guild_conf
        .has_privileges(&pool, role, &[Privilege::Admin, Privilege::Manager])
        .await
        .unwrap());

    Ok(())
}

#[apply(db_test!)]
async fn test_grant_any_privilege(pool: PgPool) -> Result<()> {
    let guild_conf = GuildConfig::from(FIRST_ID);
    let role = RoleId(1234567);
    //any other priv than Admin
    guild_conf
        .grant_privilege(&pool, role, Privilege::Event)
        .await
        .unwrap();

    assert!(guild_conf
        .has_privilege(&pool, role, Privilege::Event)
        .await
        .unwrap());
    Ok(())
}

#[apply(db_test!)]
async fn test_deny_admin_privilege(pool: PgPool) -> Result<()> {
    let guild_conf = GuildConfig::from(FIRST_ID);
    guild_conf
        .deny_privilege(&pool, FIRST_PRIV_ADMIN[0], Privilege::Admin)
        .await
        .unwrap();
    assert!(!guild_conf
        .has_privilege(&pool, FIRST_PRIV_ADMIN[0], Privilege::Admin)
        .await
        .unwrap());
    Ok(())
}

#[apply(db_test!)]
async fn test_have_privilege(pool: PgPool) -> Result<()> {
    let guild_config = GuildConfig::from(FIRST_ID);
    assert!(guild_config
        .have_privilege(
            &pool,
            &[FIRST_PRIV_MANAGER[0], FIRST_PRIV_MANAGER[1]],
            Privilege::Manager
        )
        .await
        .unwrap());
    Ok(())
}

#[apply(db_test!)]
async fn test_has_privilege(pool: PgPool) -> Result<()> {
    let guild_config = GuildConfig::from(FIRST_ID);
    assert!(guild_config
        .has_privilege(&pool, FIRST_PRIV_MANAGER[0], Privilege::Manager)
        .await
        .unwrap());
    Ok(())
}

#[apply(db_test!)]
async fn test_has_privileges(pool: PgPool) -> Result<()> {
    let guild_config = GuildConfig::from(FIRST_ID);
    assert!(guild_config
        .has_privileges(
            &pool,
            FIRST_PRIV_ADMIN[0],
            &[Privilege::Admin, Privilege::Manager]
        )
        .await
        .unwrap());
    Ok(())
}

#[apply(db_test!)]
async fn test_get_privileges_for(pool: PgPool) -> Result<()> {
    let guild_config = GuildConfig::from(FIRST_ID);
    assert_eq!(
        guild_config
            .get_privileges_for(&pool, FIRST_PRIV_ADMIN[0])
            .await
            .unwrap(),
        vec![Privilege::Admin, Privilege::Manager]
    );
    Ok(())
}
