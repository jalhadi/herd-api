use std::fmt;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serde::Serialize;
use serde_json::{Value};

use crate::pagination::Paginate;
use crate::models;
use crate::db::DbPool;
use crate::utils::{instant_to_seconds};

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

#[derive(Debug, Serialize)]
pub struct LogType {
    pub account_id: String,
    pub level: String,
    pub data: Option<Value>,
    pub created_at: u64,
}

#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    items: Vec<T>,
    total_pages: i64
}

pub fn paginated_logs<'a>(
    account_id: &'a str,
    page_number: Option<u32>,
    page_size: Option<u32>,
    conn: &PgConnection,
) -> Result<Paginated<LogType>, diesel::result::Error> {
    use crate::schema::logs;

    let page = match page_number {
        Some(p) => p,
        None => 1
    };
    let limit = match page_size {
        Some(p) => p,
        None => 10,
    };

    let (results, total_pages) = logs::table
        .order(logs::dsl::created_at.desc())
        .filter(logs::dsl::account_id.eq(account_id))
        .paginate(page as i64)
        .per_page(limit as i64)
        .load_and_count_pages::<models::Log>(conn)?;


    let mut all_logs = Vec::new();
    for log in results {
        all_logs.push(
            LogType {
                account_id: log.account_id,
                level: log.level,
                data: log.data,
                created_at: instant_to_seconds(log.created_at)
            }
        );
    }

    Ok(Paginated {
        items: all_logs,
        total_pages,
    })
}
