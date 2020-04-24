#[macro_use]
extern crate diesel;

use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize};
use actix_web_httpauth::middleware::HttpAuthentication;

mod auth;
mod db;
mod websocket;

pub mod schema;
pub mod models;

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// TODO: better error handling
async fn ws_index(pool: web::Data<db::DbPool>, r: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    // Validate websocket connection
    let account_id = auth::authenticate_websocket_connection(&r).await?;

    let device_id: &str = r.headers().get("Device-Id").unwrap().to_str().unwrap();
    let device_type_id: &str = r.headers().get("Device-Type-Id").unwrap().to_str().unwrap();
    let conn = pool.get().expect("Failed to get a db connection");

    db::create_device(
        device_id,
        device_type_id,
        &conn
    ).expect("Failed to create device");

    let res = ws::start(websocket::WebSocket::new(account_id, device_id.to_string(), pool), &r, stream);
    res
}

#[derive(Debug, Deserialize)]
struct DeviceTypePost {
    name: String,
    description: Option<String>,
}

#[allow(unused)]
async fn device_types_post(pool: web::Data<db::DbPool>, r: HttpRequest, body: web::Json<DeviceTypePost>) -> Result<HttpResponse, Error> {
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();
    let conn = pool.get().expect("Failed to get a db connection");

    db::create_device_type(
        &body.name,
        account_id,
        body.description.as_deref(),
        &conn
    );
    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Deserialize)]
struct ModulePost {
    parent_module_id: Option<String>,
    device_type_id: String,
    name: String,
    description: Option<String>,
}

#[allow(unused)]
async fn modules_post(pool: web::Data<db::DbPool>, body: web::Json<ModulePost>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");

    db::create_module(
        &body.name,
        &body.device_type_id,
        body.parent_module_id.as_deref(),
        body.description.as_deref(),
        &conn
    );

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Deserialize)]
struct ComponentPost {
    module_type_id: String,
    name: String,
    description: Option<String>,
}

#[allow(unused)]
async fn components_post(pool: web::Data<db::DbPool>, body: web::Json<ComponentPost>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");

    db::create_component(
        &body.name,
        &body.module_type_id,
        body.description.as_deref(),
        &conn
    );

    Ok(HttpResponse::Ok().finish())
}

#[derive(Debug, Deserialize)]
struct GetModule {
    device_type_id: String,
}

async fn get_module_tree(pool: web::Data<db::DbPool>, info: web::Path<GetModule>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let modules = db::get_device_modules(
        &info.device_type_id,
        account_id,
        &conn,
    );
    match modules {
        Ok(result) => Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&result).unwrap())),
        Err(_) => Ok(HttpResponse::Unauthorized().finish()),
    }

    // Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&modules).unwrap()))
}

async fn get_device_types(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let device_types = db::get_device_types(account_id, &conn);
    match device_types {
        Ok(result) => Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&result).unwrap())),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

/*
    routes needed
    POST routes:
    - device_types
    - modules
    - components

    GET routes:
    - get device_type tree
    - get events for device_type
    - get events for module
    - get events for component
*/

use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::basic::BasicAuth;

async fn validator(
    req: ServiceRequest,
    credentials: BasicAuth,
) -> Result<ServiceRequest, Error> {
    let res = auth::authenticate_connection(credentials).await;
    match res {
        Ok(_) => Ok(req),
        Err(e) => Err(e)
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    dotenv::dotenv().ok();

    HttpServer::new(|| {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .data(db::init_pool())
            .service(web::resource("/").route(web::get().to(health_check)))
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            // .wrap(HttpAuthentication::basic(validator))
            .service(web::resource("/device_types")
                .route(web::post().to(device_types_post))
                .route(web::get().to(get_device_types)))
            .service(
                web::resource("/device_types/{device_type_id}/module_tree")
                    .route(web::get().to(get_module_tree))
            )
            .service(web::resource("/modules").route(web::post().to(modules_post)))
            .service(web::resource("/components").route(web::post().to(components_post)))
    })
    // start http server on 127.0.0.1:8080
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
