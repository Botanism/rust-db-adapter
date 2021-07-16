use sqlx::postgres::PgPool;
use std::env;
use std::fmt::Write;

pub mod guild;

pub async fn establish_connection() -> PgPool {
    dotenv::dotenv().ok();
    PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL was not set"))
        .await
        .expect("Could not establish connection")
}

pub(crate) fn as_pg_array(ids: &[i64]) -> String {
    let mut array = String::new();
    if ids.is_empty() {
        array.push_str("'{}'");
        return array;
    }
    write!(array, "'{{").unwrap();
    for int in ids {
        write!(array, "{},", int).unwrap();
    }
    array.pop(); //removing the trailing comma
    write!(array, "}}'").unwrap();
    array
}

/*
pub(crate) fn to_comma_separated(ids: &[i64]) -> String {
    let mut array = String::new();
    if ids.is_empty() {
        return array;
    }
    for int in ids {
        write!(array, "{},", int).unwrap()
    }
    array.pop(); //removing the trailing comma

    array
}
*/
