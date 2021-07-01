//based on https://github.com/almetica/almetica/blob/9d9688d3d1ddddae2594ed18fe78ac6b5718d1e7/src/model.rs#L375
// licensed under AGPL 3.0 by almetica

pub mod db_test_interface {
    use std::env;
    use std::panic;

    use dotenv::dotenv;
    use rand::{thread_rng, Rng};
    use sqlx::{Connection, PgConnection, Result};
    use tokio::runtime::Runtime;

    pub fn db_session<F>(test: F) -> Result<()>
    where
        //we only pass the url instead of the connection because creating it requires async
        F: FnOnce(&str) -> Result<()> + panic::UnwindSafe,
    {
        dotenv().ok();
        let db_url = env::var("TEST_DB_URL").expect("TEST_DB_URL was not set");

        // TODO: ugly! try to find a better way to do the following
        let this_runtime = Runtime::new().unwrap();
        let db_name = this_runtime.block_on(async { db_setup(&db_url).await })?;

        let result = panic::catch_unwind(|| {
            let db_string = format!("{}/{}", db_url, db_name);
            if let Err(e) = test(&db_string) {
                panic!("Error occured while executing test: {:?}", e)
            }
        });

        this_runtime.block_on(async { db_teardown(&db_url, &db_name).await })?;

        assert!(result.is_ok());

        Ok(())
    }

    //we create a db to only for one test
    async fn db_setup(db_url: &str) -> Result<String> {
        let mut name = String::from("BotanistTest_");
        let random_id: u128 = thread_rng().gen();
        name.push_str(random_id.to_string().as_str());

        let mut conn = PgConnection::connect(&db_url).await?;
        sqlx::query(&format!("CREATE DATABASE {};", name))
            //executor is only impl for &mut Connection
            .execute(&mut conn)
            .await?;

        Ok(name)
    }

    ///we drop tehe db after testing
    async fn db_teardown(db_url: &str, name: &str) -> Result<()> {
        let mut conn = PgConnection::connect(&db_url).await?;

        sqlx::query(&format!("DROP DATABSE {};", name))
            .execute(&mut conn)
            .await?;
        conn.close().await?;

        Ok(())
    }
}
