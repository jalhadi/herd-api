use serde_json::Value;
use diesel::prelude::*;
use std::time::SystemTime;
use std::vec::Vec;

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
    Pool::builder()
        .max_size(4)
        .build(manager)
        .expect("db pool")
}

/*
    Functions needed:
        - insert device_type
        - insert device
        - insert module
        - insert component
*/

fn generate_random_uuid () -> String {
    Uuid::new_v4().to_simple().to_string()
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

pub fn create_module<'a>(
    name: &'a str,
    device_type_id: &'a str,
    parent_id: Option<&'a str>,
    description: Option<&'a str>,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::modules;
    let id = format!("mod_{}", generate_random_uuid());
    let new_module = models::NewModule {
        id: &id,
        name,
        device_type_id,
        parent_id,
        description,
    };

    diesel::insert_into(modules::table).values(&new_module).execute(conn)?;    
    Ok(())
}

pub fn create_component<'a>(
    name: &'a str,
    module_type_id: &'a str,
    description: Option<&'a str>,
    conn: &PgConnection,
) -> Result<(), diesel::result::Error> {
    use crate::schema::components;
    let id = format!("com_{}", generate_random_uuid());
    let new_component = models::NewComponent {
        id: &id,
        name,
        description,
        module_type_id,
    };

    diesel::insert_into(components::table).values(&new_component).execute(conn)?;    
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
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}

pub fn insert_event<'a>(
    component_id: &'a str,
    device_id: &'a str,
    data: Value,
    conn: &PgConnection
) -> Result<(), diesel::result::Error> {
    use crate::schema::component_events;

    let new_item = models::NewComponentEvent {
        component_id,
        device_id,
        data
    };

    diesel::insert_into(component_events::table).values(&new_item).execute(conn)?;

    Ok(())
}

pub fn get_device_types<'a>(
    account_id: &'a str,
    conn: &PgConnection,
) -> Result<Vec<models::DeviceType>, diesel::result::Error>{
    use crate::schema::device_types::dsl;

    let result = dsl::device_types
        .filter(dsl::account_id.eq(account_id))
        .load::<models::DeviceType>(conn)
        .expect("An error occurred");

    Ok(result)
}

#[derive(Debug, Serialize)]
pub struct ModuleTree {
    pub id: String,
    pub device_type_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub components: Vec<models::Component>,
    pub children: Vec<ModuleTree>,
}

#[derive(Debug, Serialize)]
pub struct DeviceTree {
    pub modules: Vec<ModuleTree>,
}

pub fn get_components<'a>(
    module_id: &'a str,
    conn: &PgConnection,
) -> Vec<models::Component> {
    use crate::schema::components::dsl;

    dsl::components
        .filter(dsl::module_type_id.eq(module_id))
        .load::<models::Component>(conn)
        .expect("An error occurred")
}

pub fn get_modules<'a>(
    parent_id: Option<&'a str>,
    conn: &PgConnection,
) -> Vec<ModuleTree> {
    use crate::schema::modules::dsl;

    let modules = match parent_id {
        Some(x) => dsl::modules
            .filter(dsl::parent_id.eq(x))
            .load::<models::Module>(conn)
            .expect("An error occurred"),
        None => dsl::modules
            .filter(dsl::parent_id.is_null())
            .load::<models::Module>(conn)
            .expect("An error occurred"),
    };

    let mut all_module_components = Vec::new();
    for module in modules {
        let module_components = get_components(&module.id, conn);
        let children = get_modules(Some(&module.id), conn);
        all_module_components.push(
            ModuleTree {
                id: module.id,
                device_type_id: module.device_type_id,
                name: module.name,
                description: module.description,
                created_at: module.created_at,
                updated_at: module.updated_at,
                components: module_components,
                children
            }
        );
    }
    all_module_components
}

pub fn get_device_modules<'a>(
    device_type_id: &'a str,
    conn: &PgConnection,
) ->  Vec<ModuleTree> {
    use crate::schema::modules::dsl;

    // TODO: check that device_type_id belongs
    // to the current account_id
    let modules = dsl::modules
        .filter(dsl::parent_id.is_null())
        .filter(dsl::device_type_id.eq(device_type_id))
        .load::<models::Module>(conn)
        .expect("An error occurred");

    let mut all_module_components = Vec::new();
    for module in modules {
        let module_components = get_components(&module.id, conn);
        let children = get_modules(Some(&module.id), conn);
        all_module_components.push(
            ModuleTree {
                id: module.id,
                device_type_id: module.device_type_id,
                name: module.name,
                description: module.description,
                created_at: module.created_at,
                updated_at: module.updated_at,
                components: module_components,
                children
            }
        );
    }
    all_module_components
}
