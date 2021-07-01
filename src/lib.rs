use dotenv;
use sqlx::postgres::PgPool;
use std::env;

pub async fn establish_connection() -> PgPool {
    dotenv::dotenv().ok();
    PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL was not set"))
        .await
        .expect("Could not establish connection")
}
