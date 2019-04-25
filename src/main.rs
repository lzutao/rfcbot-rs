#![deny(rust_2018_idioms)]
#![feature(never_type)]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate maplit;

#[macro_use]
mod macros;

mod config;
mod domain;
mod error;
mod github;
mod nag;
mod scraper;
mod server;
mod teams;
mod utils;

use chrono::Local;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use env_logger::LogBuilder;
use log::LogRecord;

use crate::config::CONFIG;

fn main() {
    // init environment variables, CLI, and logging
    dotenv::dotenv().ok();

    LogBuilder::new()
        .format(|rec: &LogRecord<'_>| {
            let loc = rec.location();
            format!(
                "[{} {}:{} {}] {}",
                rec.level(),
                loc.module_path(),
                loc.line(),
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                rec.args()
            )
        })
        .parse(&std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init()
        .unwrap();

    debug!("Logging initialized.");
    let _ = CONFIG.check();
    let _ = DB_POOL.get().expect("Unable to test connection pool.");

    // we want to panic if we're unable to find any of the usernames
    {
        let teams = teams::SETUP.read().unwrap();
        let parsed_teams = teams.team_labels().collect::<Vec<_>>();
        info!("parsed teams: {:?}", parsed_teams);
    }

    teams::start_updater_thread();

    // FIXME(anp) need to handle panics in both the listeners and crash the server
    let _ = scraper::start_scraping();
    let _server_handle = server::serve();

    // block
    //server_handle.join().expect("problem running server!").expect("problem while running server");
}

// initialize the database connection pool
lazy_static! {
    pub static ref DB_POOL: Pool<ConnectionManager<PgConnection>> = {
        info!("Initializing database connection pool.");

        let manager = ConnectionManager::<PgConnection>::new(CONFIG.db_url.clone());

        match Pool::builder().max_size(CONFIG.db_pool_size).build(manager) {
            Ok(p) => {
                info!("DB connection pool established.");
                p
            }
            Err(why) => {
                error!("Failed to establish DB connection pool: {}", why);
                panic!("Error creating connection pool.");
            }
        }
    };
}
