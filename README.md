# db-adapter

A wrapper around Botanist database scheme

## Objective

This crate was built with the intent to provide a safe wrapper around Botanist's
database scheme. This way invariants remain true at all times, no other tool can
mess the data in an incorrect way.

Centralizing the interactions with the database also allows finer control over
iterations of its scheme. On that note changes in the scheme are done through migrations scripts
(see the `migrations`) folder. Hence the setup of the databse is made very simple with
[`sqlx`'s cli tool]. Moreover deviations from the scheme provided by the migration scripts
will be detected by the tests (see `tests/framework` and [migrate]).

Finally providing a rust library allows [db_adapter] to provide useful abstractions.

## Setup
Setup is intended to be as simple as possible so if you find some way to simplify a step please open
an issue.
First of all make sure you have a posgresql database up and running. Search online for walkthroughs
 if you don't know how. Then rename `.env-example` to `.env` and enter make sure you place your values
in it.
Now install [sqlx-cli] and run the migrations using `sqlx migrate run`. If you set up the DB and `.env`
correctly you should be good to go!
If you're only using the library you don't need to do anuything else but you could still
run the tests just in case: `cargo t`.

## Developement

To contribute to [db_adapter] you should setup the test environement. In addition to the previous section's
steps you should setup another DB for the tests and refer it in `.env`. From there you can dive in the code!
Just make sure you don't break any invariants and remember to respect semver. You are also expected
to document and tests any item added to the public API.

[sqlx-cli]: https://github.com/launchbadge/sqlx/tree/master/sqlx-cli
[`sqlx`'s cli tool]: https://github.com/launchbadge/sqlx/tree/master/sqlx-cli
[db_adapter]: https://github.com/Botanism/rust-db-adapter/
[migrate]: https://docs.rs/sqlx/0.5.5/sqlx/migrate/index.html