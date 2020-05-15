use std::fmt;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serde::Serialize;
use serde_json::{Value};

use crate::models;
use crate::db::DbPool;

pub enum LogLevel {
    Info,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Info =>  write!(f, "info"),
            LogLevel::Error =>  write!(f, "error"),
        }
    }
}

pub fn log<'a>(
    account_id: &'a str,
    log_level: LogLevel,
    data: Value,
    pool: &DbPool,
) -> () {
    match pool.get() {
        Ok(conn) => {
            let _ = insert_log(
                account_id,
                log_level,
                Some(data),
                &conn
            );
        },
        Err(e) => println!("Error getting db connection: {:?}", e),
    };
}

pub fn insert_log<'a>(
    account_id: &'a str,
    log_level: LogLevel,
    data: Option<Value>,
    conn: &PgConnection
) -> Result<(), diesel::result::Error> {
    use crate::schema::logs;

    let new_log = models::NewLog {
        account_id,
        level: &log_level.to_string(),
        data,
    };

    diesel::insert_into(logs::table)
        .values(&new_log)
        .execute(conn)?;
    Ok(())
}