//! A wrapper around Botanist database scheme
//!
//! [![rust-db-adapter](https://github.com/Botanism/rust-db-adapter/actions/workflows/rust.yml/badge.svg)](https://github.com/Botanism/rust-db-adapter/actions/workflows/rust.yml)
//!
//! # Objective
//!
//! This crate was built with the intent to provide a safe wrapper around Botanist's
//! database scheme. This way invariants remain true at all times, no other tool can
//! mess the data in an incorrect way.
//!
//! Centralizing the interactions with the database also allows finer control over
//! iterations of its scheme. On that note changes in the scheme are done through migrations scripts
//! (see the `migrations`) folder. Hence the setup of the database is made very simple with
//! [`sqlx`]'s cli tool. Moreover deviations from the scheme provided by the migration scripts
//! will be detected by the tests (see `tests/framework` and [`mod@sqlx::migrate`]).
//!
//! Finally providing a rust library allows [db_adapter] to provide useful abstractions.
//!
//! # Setup
//! Setup is intended to be as simple as possible so if you find some way to simplify a step please open
//! an issue.
//! First of all make sure you have a postgresql database up and running. Search online for walkthroughs
//!  if you don't know how. Then rename `.env-example` to `.env` and enter make sure you place your values
//! in it.
//! Now install [sqlx-cli] and run the migrations using `sqlx migrate run`. If you set up the DB and `.env`
//! correctly you should be good to go!
//! If you're only using the library you don't need to do anything else but you could still
//! run the tests just in case: `cargo t`.
//!
//! # Developement
//!
//! To contribute to [db_adapter] you should setup the test environement. In addition to the previous section's
//! steps you should setup another DB for the tests and refer it in `.env`. From there you can dive in the code!
//! Just make sure you don't break any invariants and remember to respect semver. You are also expected
//! to document and tests any item added to the public API.
//!
//! [sqlx-cli]: https://github.com/launchbadge/sqlx/tree/master/sqlx-cli
//! [db_adapter]: [`self`]

#[cfg(feature = "net")]
use rocket::{
    http::ContentType,
    request::Request,
    response::{self, Responder, Response},
};
#[cfg(feature = "net")]
use std::io::Cursor;

pub use sqlx::postgres::PgPool;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::env;
use std::fmt::Write;
use thiserror::Error;

pub mod guild;
pub mod slap;
#[cfg(test)]
mod tests;

/// Creates a [connection pool] to the database
///
/// # Panic
/// Panics if `DATABASE_URL` is not set or if the connection could not be established.
///
/// [connection pool]: sqlx::postgres::PgPool
pub async fn establish_connection() -> PgPool {
    dotenv::dotenv().ok();
    PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL was not set"))
        .await
        .expect("Could not establish connection")
}

/// Wrapper around all errors coming from the crate
#[derive(Debug, Error)]
pub enum AdapterError {
    /// [`sqlx::Error`] errors
    ///
    /// The crate uses [`sqlx`] under the hood to communicate with the DBs.
    /// If the later fails for any reason the error is relayed.
    #[error("could not execute querry")]
    SqlxError(#[from] sqlx::Error),
    /// Errors with guilds' configuration
    #[error("guild configuration error")]
    GuildError(#[from] guild::GuildConfigError),
}

#[cfg(feature = "net")]
impl<'r, 'o: 'r> Responder<'r, 'o> for AdapterError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let why = "not good";
        Response::build()
            .header(ContentType::JSON)
            .sized_body(why.len(), Cursor::new(why))
            .ok()
    }
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

pub(crate) fn stringify_option<'a, T: std::fmt::Display>(option: Option<T>) -> Cow<'a, str> {
    match option {
        Some(value) => Cow::Owned(format!("'{}'", value)),
        None => Cow::Borrowed("NULL"),
    }
}

pub(crate) fn from_i64<I: From<u64>>(int: i64) -> I {
    u64::try_from(int).unwrap().into()
}

pub(crate) fn to_i64<I: Into<u64>>(id: I) -> i64 {
    i64::try_from(id.into()).unwrap()
}
