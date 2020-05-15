use diesel::dsl::{exists, select};
use diesel::prelude::*;
use std::time::SystemTime;
use std::vec::Vec;

use diesel_migrations::run_pending_migrations;
use diesel::pg::PgConnection;
use diesel::r2d2::{ Pool, ConnectionManager };
use std::env;
use uuid::Uuid;
use serde::{Serialize};

use crate::models;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

fn database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

pub fn init_pool() -> DbPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url());

    let pool = Pool::builder()
        .max_size(4)
        .build(manager)
        .expect("db pool");

    let conn = pool.get().expect("Failed to get a db connection");
    // TODO: fail loudly when migrations fail
    run_pending_migrations(&conn).expect("Failed to run migrations");

    return pool;
}

fn generate_random_uuid () -> String {
    Uuid::new_v4().to_simple().to_string()
}

fn instant_to_seconds(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error in getting time bucket")
        .as_secs()
}

pub fn create_device_type<'a>(
    name: &'a str,
    account_id: &'a str,
    description: Option<&'a str>,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::device_types;
    let id = format!("devt_{}", generate_random_uuid());
    let new_device_type = models::NewDeviceType {
        name,
        account_id,
        id: &id,
        description,
    };
    diesel::insert_into(device_types::table).values(&new_device_type).execute(conn)?;

    Ok(())
}

pub fn create_device<'a>(
    id: &'a str,
    device_type_id: &'a str,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::devices;

    let new_device = models::NewDevice {
        id,
        device_type_id,
    };

    diesel::insert_into(devices::table)
        .values(&new_device)
        // TODO: this is broken, on conflist do nothing
        // surpresses all errors
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct DeviceType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
}

pub fn get_device_types<'a>(
    account_id: &'a str,
    conn: &PgConnection,
) -> Result<Vec<DeviceType>, diesel::result::Error>{
    use crate::schema::device_types::dsl;

    let result = dsl::device_types
        .filter(dsl::account_id.eq(account_id))
        .load::<models::DeviceType>(conn)
        .expect("An error occurred");

    let mut all_device_types = Vec::new();
    for device_type in result {
        all_device_types.push(
            DeviceType {
                id: device_type.id.clone(),
                name: device_type.name,
                description: device_type.description,
                created_at: instant_to_seconds(device_type.created_at),
            }
        );
    }

    Ok(all_device_types)
}

#[derive(Debug, Serialize)]
pub struct TopicType {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

pub fn get_topics<'a>(
    account_id: &'a str,
    conn: &PgConnection,
) -> Vec<TopicType> {
    use crate::schema::topics::dsl;

    let result = dsl::topics
        .filter(dsl::account_id.eq(account_id))
        .load::<models::Topic>(conn)
        .expect("An error occurred");

    let mut all_topics = Vec::new();
    for topic in result {
        all_topics.push(
            TopicType {
                id: topic.id,
                account_id: topic.account_id,
                name: topic.name,
                description: topic.description,
                created_at: instant_to_seconds(topic.created_at),
                updated_at: instant_to_seconds(topic.updated_at),
            }
        );
    }

    all_topics
}

pub fn create_topic<'a>(
    name: &'a str,
    account_id: &'a str,
    description: Option<&'a str>,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::topics;

    let id = format!("top_{}", generate_random_uuid());
    let new_topic = models::NewTopic {
        id: &id,
        name,
        account_id,
        description,
    };

    diesel::insert_into(topics::table)
        .values(&new_topic)
        .execute(conn)?;
    Ok(())
}

pub fn topic_relation_exists<'a>(
    account_id: &'a str,
    topic_id: &'a str,
    conn: &PgConnection
) -> bool {
    use crate::schema::topics::dsl;

    let result = select(exists(
        dsl::topics.filter(dsl::id.eq(topic_id)).filter(dsl::account_id.eq(account_id))))
        .get_result::<bool>(conn);
    match result {
        Ok(true) => true,
        _ => false,
    }
}

pub struct TopicRelation {
    pub id: String,
    pub account_id: String,
}

pub fn get_all_topic_relations<'a>(
    conn: &PgConnection
) -> Result<Vec<TopicRelation>, diesel::result::Error> {
    use crate::schema::topics::dsl;

    let result = dsl::topics
        .select((dsl::id, dsl::account_id))
        .load::<(String, String)>(conn)?;

    let mut relations = Vec::new();
    for item in result {
        relations.push(
            TopicRelation {
                id: item.0,
                account_id: item.1
            }
        );
    }
    Ok(relations)
}

pub fn create_webhook<'a>(
    account_id: &'a str,
    url: &'a str,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::webhooks;

    let new_webhook = models::NewWebhook {
        account_id,
        url
    };

    diesel::insert_into(webhooks::table)
        .values(&new_webhook)
        .execute(conn)?;

    Ok(())
}

pub fn delete_webhook<'a>(
    webhook_id: i32,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::webhook_topics::dsl as webhook_topics_dsl;
    use crate::schema::webhooks::dsl as webhooks_dsl;

    // Need to delete all topic associations before deleting
    // the webhook
    diesel::delete(webhook_topics_dsl::webhook_topics.filter(webhook_topics_dsl::webhook_id.eq(&webhook_id)))
        .execute(conn)?;

    // Delete the webhook
    diesel::delete(webhooks_dsl::webhooks.filter(webhooks_dsl::id.eq(&webhook_id)))
        .execute(conn)?;

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct WebhookType {
    pub id: i32,
    pub account_id: String,
    pub url: String,
    pub created_at: u64,
    pub updated_at: u64,
}

pub fn get_webhooks<'a>(
    account_id: &'a str,
    conn: &PgConnection,
) -> Vec<WebhookType> {
    use crate::schema::webhooks::dsl;

    let result = dsl::webhooks
        .filter(dsl::account_id.eq(account_id))
        .load::<models::Webhook>(conn)
        .expect("An error occurred");

    let mut all_webhooks = Vec::new();
    for webhook in result {
        all_webhooks.push(
            WebhookType {
                id: webhook.id,
                account_id: webhook.account_id,
                url: webhook.url,
                created_at: instant_to_seconds(webhook.created_at),
                updated_at: instant_to_seconds(webhook.updated_at),
            }
        );
    }

    all_webhooks
}

pub fn create_webhook_topic<'a>(
    webhook_id: i32,
    topic_id: &'a str,
    conn: &PgConnection
) -> Result<(), diesel::result::Error> {
    use crate::schema::webhook_topics;

    let new_webhook_topic = models::NewWebhookTopic {
        webhook_id,
        topic_id
    };

    diesel::insert_into(webhook_topics::table)
        .values(&new_webhook_topic)
        .execute(conn)?;

    Ok(())
}

pub fn delete_webhook_topic<'a>(
    id: i32,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::webhook_topics::dsl;

    diesel::delete(dsl::webhook_topics.filter(
        dsl::id.eq(id)))
        .execute(conn)?;

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct WebhookTopic {
    pub id: i32,
    pub webhook_id: i32,
    pub topic_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

pub fn get_webhook_topics<'a>(
    webhook_id: i32,
    conn: &PgConnection
) -> Vec<WebhookTopic> {
    use crate::schema::webhook_topics;
    use crate::schema::topics;

    let join = webhook_topics::table.inner_join(topics::table);

    let result = join
        .filter(webhook_topics::dsl::webhook_id.eq(webhook_id))
        .select((
            webhook_topics::dsl::id,
            webhook_topics::dsl::webhook_id,
            topics::dsl::id,
            topics::dsl::name,
            topics::dsl::description,
            topics::dsl::created_at,
            topics::dsl::updated_at
        ))
        .load::<(i32, i32, String, String, Option<String>, SystemTime, SystemTime)>(conn)
        .expect("An error occured.");

    let mut all_topics = Vec::new();
    for topic in result {
        all_topics.push(
            WebhookTopic {
                id: topic.0,
                webhook_id: topic.1,
                topic_id: topic.2,
                name: topic.3,
                description: topic.4,
                created_at: instant_to_seconds(topic.5),
                updated_at: instant_to_seconds(topic.6)
            }
        );
    }

    all_topics
}

pub fn get_all_webhook_topics<'a>(conn: &PgConnection) -> Result<Vec<(String, String)>, diesel::result::Error>  {
    use crate::schema::webhook_topics;
    use crate::schema::webhooks;

    let join = webhook_topics::table.inner_join(webhooks::table);
    join
        .select((webhook_topics::dsl::topic_id, webhooks::dsl::url))
        .load::<(String, String)>(conn)
}
