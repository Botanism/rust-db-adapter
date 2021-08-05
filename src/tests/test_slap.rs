use super::framework::{
    db_test_interface::{db_session, db_test},
    guild_test_info::FIRST_ID,
    slap_test_info::*,
};
use crate::slap::*;
use macro_rules_attribute::apply;
use serenity::model::id::MessageId;
use sqlx::{PgPool, Result};
use tokio_stream::StreamExt;

//sr stands for SlapReport and can be prefixed with `g` for Guild or `m` for Member

#[apply(db_test!)]
async fn sr_get(conn: PgPool) -> Result<()> {
    let report = SlapReport::get(&conn, FIRST_SENTENCE).await.unwrap();
    assert_eq!(report, Some(assemble_from_test!("FIRST")));
    Ok(())
}

#[apply(db_test!)]
async fn msr_len(conn: PgPool) -> Result<()> {
    let record = MemberSlapRecord::from((FIRST_ID, FIRST_OFFENDER));
    assert_eq!(record.len(&conn).await.unwrap(), 2);
    Ok(())
}

#[apply(db_test!)]
async fn msr_slaps(conn: PgPool) -> Result<()> {
    let record = MemberSlapRecord::from((FIRST_ID, FIRST_OFFENDER));
    let reports = vec![assemble_from_test!("FIRST"), assemble_from_test!("SECOND")];
    assert_eq!(
        record
            .slaps(&conn)
            .map(|res| res.unwrap())
            .collect::<Vec<SlapReport>>()
            .await,
        reports
    );
    Ok(())
}

#[apply(db_test!)]
async fn msr_new_slap(conn: PgPool) -> Result<()> {
    let record = MemberSlapRecord::from((FIRST_ID, FIRST_OFFENDER));
    let sentence = MessageId(5864);
    let report = record
        .new_slap(&conn, sentence, Enforcer::Community, None)
        .await
        .unwrap();
    assert_eq!(
        Some(report),
        SlapReport::get(&conn, sentence).await.unwrap()
    );

    Ok(())
}

#[apply(db_test!)]
async fn gsr_slaps(conn: PgPool) -> Result<()> {
    let record = GuildSlapRecord::from(FIRST_ID);
    let reports = vec![
        assemble_from_test!("FIRST"),
        assemble_from_test!("SECOND"),
        assemble_from_test!("FOURTH"),
    ];
    assert_eq!(
        record
            .slaps(&conn)
            .map(|res| res.unwrap())
            .collect::<Vec<SlapReport>>()
            .await,
        reports
    );
    Ok(())
}

#[apply(db_test!)]
async fn gsr_members(conn: PgPool) -> Result<()> {
    let record = GuildSlapRecord::from(FIRST_ID);
    let members = record
        .members(&conn)
        .map(|res| res.unwrap())
        .collect::<Vec<MemberSlapRecord>>()
        .await;
    assert!(members.contains(&MemberSlapRecord::from((FIRST_ID, FIRST_OFFENDER))));
    assert!(members.contains(&MemberSlapRecord::from((FIRST_ID, FOURTH_OFFENDER))));

    Ok(())
}
