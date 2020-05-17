use super::schema::device_types;
use super::schema::devices;
use super::schema::topics;
use super::schema::webhooks;
use super::schema::webhook_topics;
use super::schema::logs;

use std::time::SystemTime;
use serde::{Serialize};
use serde_json::{Value};

#[derive(Queryable, Serialize)]
pub struct DeviceType {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[table_name = "device_types"]
pub struct NewDeviceType<'a> {
    pub id: &'a str,
    pub account_id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Queryable)]
pub struct Devices {
    pub id: String,
    pub device_type_id: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Insertable, Debug)]
#[table_name = "devices"]
pub struct NewDevice<'a> {
    pub id: &'a str,
    pub device_type_id: &'a str,
}

#[derive(Queryable)]
pub struct Topic {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Insertable, Debug)]
#[table_name = "topics"]
pub struct NewTopic<'a> {
    pub id: &'a str,
    pub account_id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
}

#[derive(Queryable)]
pub struct Webhook {
    pub id: i32,
    pub account_id: String,
    pub url: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Insertable, Debug)]
#[table_name = "webhooks"]
pub struct NewWebhook<'a> {
    pub account_id: &'a str,
    pub url: &'a str,
}

#[derive(Queryable)]
pub struct WebhookTopic {
    pub id: i32,
    pub endpoint_id: i32,
    pub topic_id: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Insertable, Debug)]
#[table_name = "webhook_topics"]
pub struct NewWebhookTopic<'a> {
    pub webhook_id: i32,
    pub topic_id: &'a str,
}

#[derive(Queryable)]
pub struct Log {
    pub id: i32,
    pub account_id: String,
    pub level: String,
    pub data: Option<Value>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Insertable, Debug)]
#[table_name = "logs"]
pub struct NewLog<'a> {
    pub account_id: &'a str,
    pub level: &'a str,
    pub data: Option<Value>,
}
