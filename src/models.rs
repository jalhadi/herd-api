use super::schema::component_events;
use super::schema::device_types;
use super::schema::modules;
use super::schema::components;
use super::schema::devices;

use std::time::SystemTime;
use serde_json::Value;
use diesel::pg::types::sql_types::Jsonb;
use serde::{Serialize};

#[derive(Queryable)]
pub struct ComponentEvent {
    pub id: i32,
    pub component_id: String,
    pub data: Jsonb,
    pub created_at: SystemTime,
    pub updatd_at: SystemTime,
}

#[derive(Insertable)]
#[table_name = "component_events"]
pub struct NewComponentEvent<'a> {
    pub component_id: &'a str,
    pub device_id: &'a str,
    pub data: Value,
}

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

#[derive(Queryable, Debug)]
pub struct Module {
    pub id: String,
    pub device_type_id: String,
    pub name: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub parent_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[table_name = "modules"]
pub struct NewModule<'a> {
    pub id: &'a str,
    pub device_type_id: &'a str,
    pub name: &'a str,
    pub parent_id: Option<&'a str>,
    pub description: Option<&'a str>,
}

#[derive(Queryable, Debug, Serialize)]
pub struct Component {
    pub id: String,
    pub module_type_id: String,
    pub name: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub description: Option<String>,
}

#[derive(Insertable)]
#[table_name = "components"]
pub struct NewComponent<'a> {
    pub id: &'a str,
    pub module_type_id: &'a str,
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

#[derive(Insertable)]
#[table_name = "devices"]
pub struct NewDevice<'a> {
    pub id: &'a str,
    pub device_type_id: &'a str,
}
