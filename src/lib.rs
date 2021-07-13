use sqlx::postgres::PgPool;
use std::env;
use std::fmt::Write;
use thiserror::Error;

pub mod guild;

pub async fn establish_connection() -> PgPool {
    dotenv::dotenv().ok();
    PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL was not set"))
        .await
        .expect("Could not establish connection")
}

pub(crate) fn as_pg_array(vec: Vec<i64>) -> String {
    let mut array = String::new();
    if vec.is_empty() {
        return String::from("'{}'");
    }
    write!(array, "'{{").unwrap();
    for int in vec {
        write!(array, "{},", int).unwrap();
    }
    array.pop(); //removing the trailing comma
    write!(array, "}}'").unwrap();
    array
}
