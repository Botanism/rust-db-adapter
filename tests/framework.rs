//based on https://github.com/almetica/almetica/blob/9d9688d3d1ddddae2594ed18fe78ac6b5718d1e7/src/model.rs#L375
// licensed under AGPL 3.0 by almetica

pub mod db_test_interface {
    use std::env;
    use std::panic;

    use dotenv::dotenv;
    use rand::{thread_rng, Rng};
    use sqlx::{migrate, query, Connection, PgConnection, Result};
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

        this_runtime.block_on(async { teardown_db(&db_url, &db_name).await })?;

        assert!(result.is_ok());

        Ok(())
    }

    //we create a db to only for one test
    async fn db_setup(db_url: &str) -> Result<String> {
        let mut name = String::from("BotanistTest_");
        let random_id: u128 = thread_rng().gen();
        name.push_str(random_id.to_string().as_str());
        println!("{:?}", &name);

        let mut conn = PgConnection::connect(&db_url).await?;
        // TODO: investigave why using the `query!` macro would not compile
        sqlx::query(&format!("CREATE TABLE {}", name))
            //Executor is only impl for &mut Connection
            .execute(&mut conn)
            .await?;
        apply_migrations(&mut conn).await?;
        insert_dummy(&mut conn).await?;

        Ok(name)
    }

    ///we drop the db after testing
    async fn teardown_db(db_url: &str, name: &str) -> Result<()> {
        let mut conn = PgConnection::connect(&db_url).await?;

        sqlx::query(&format!("DROP DATABASE {};", name))
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

    /// inserts some dummy values into the dabase to allow tests to be relevant
    async fn insert_dummy(conn: &mut PgConnection) -> Result<()> {
        query!("INSERT INTO guilds(id, welcome_message, goodbye_message, advertise, admin_chan, poll_chans, priv_manager, priv_admin, priv_event) VALUES (0, 'hello', NULL, true, 76543, array[5345345, 5764574], array[6675421, 2321390], array[6675421], array[46456]),(1, NULL, 'goodbye', false, 765430, array[53453450, 57645740], array[66754210, 23213900], array[66754210], array[464560])")
            .execute(conn)
            .await?;
        Ok(())
    }
}
