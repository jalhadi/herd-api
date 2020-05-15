#[macro_use]
extern crate diesel;

use serde_json::Value;
use actix;
use actix::prelude::*;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize};
use actix_web::client::Client;
use futures::future::{join_all};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::sync::mpsc::channel;

use crate::webhook_publisher::WebhookMessage;

mod auth;
mod db;
mod websocket;
mod rate_limiter;
mod publisher;
mod webhook_publisher;
mod utils;

pub mod schema;
pub mod models;

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// TODO: better error handling
async fn ws_index(
    pool: web::Data<db::DbPool>,
    r: HttpRequest,
    stream: web::Payload,
    publish: web::Data<Addr<publisher::Publisher>>
) -> Result<HttpResponse, Error> {
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

    let res = ws::start(websocket::WebSocket::new(
        account_id,
        device_id.to_string(),
        device_type_id.to_string(),
        publish.get_ref().clone(),
    ), &r, stream);
    res
}

#[derive(Debug, Deserialize)]
struct MessagePost {
    topics: Vec<String>,
    data: Value,
}

async fn message(
    r: HttpRequest,
    body: web::Json<MessagePost>,
    publish: web::Data<Addr<publisher::Publisher>>
) -> HttpResponse {
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();
    let uri = r.peer_addr();
    let sender = publisher::Sender::Address(uri);

    let time = utils::get_time().expect("An error occurred.");
    let message = websocket::Message {
        seconds_since_unix: time.seconds_since_unix,
        nano_seconds: time.nano_seconds,
        topics: body.topics.clone(),
        data: body.data.clone()
    };
    let publish_message = publisher::PublishMessage {
        sender,
        account_id: account_id.to_owned(), 
        message
    };

    publish.do_send(publish_message);

    HttpResponse::Ok().finish()
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

async fn get_topics(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let topics = db::get_topics(account_id, &conn);
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&topics).unwrap()))
}

#[derive(Debug, Deserialize)]
struct TopicsPost {
    name: String,
    description: Option<String>,
}

async fn topics_post(pool: web::Data<db::DbPool>, r: HttpRequest, body: web::Json<TopicsPost>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let create_topic_result = db::create_topic(
        &body.name,
        account_id,
        body.description.as_deref(),
        &conn
    );

    match create_topic_result {
        Ok(result) => Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&result).unwrap())),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

async fn get_webhooks(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let webhooks = db::get_webhooks(account_id, &conn);
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&webhooks).unwrap()))
}

#[derive(Debug, Deserialize)]
struct WebhooksPost {
    url: String,
}

async fn webhooks_post(pool: web::Data<db::DbPool>, r: HttpRequest, body: web::Json<WebhooksPost>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let create_webhook_result = db::create_webhook(
        account_id,
        &body.url,
        &conn
    );

    match create_webhook_result {
        Ok(result) => Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&result).unwrap())),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

async fn get_webhook_topics(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();
    let webhook_id = r.match_info().query("id").parse().unwrap();

    let webhook_topics = db::get_webhook_topics(webhook_id, &conn);
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&webhook_topics).unwrap()))
}

async fn delete_webhook_topic(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let id = r.match_info().query("id").parse().unwrap();

    let delete_result = db::delete_webhook_topic(id, &conn);

    match delete_result {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

#[derive(Debug, Deserialize)]
struct WebhookTopicsPost {
    webhook_id: i32,
    topic_ids: Vec<String>,
}

async fn webhook_topics_post(pool: web::Data<db::DbPool>, r: HttpRequest, body: web::Json<WebhookTopicsPost>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    
    // TODO: check that the current connection is authorized to add
    // a topic to the webhook_id specified, i.e. check the
    // webhooks table for the combination of webhook_id and account_id (from headers)

    for id in &body.topic_ids {
        db::create_webhook_topic(
            body.webhook_id,
            &id,
            &conn
        ).expect("An error occurred.");
    }

    Ok(HttpResponse::Ok().finish())
    // match create_webhook_topic_result {
    //     Ok(result) => Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&result).unwrap())),
    //     Err(_) => {
    //         Ok(HttpResponse::BadRequest().finish())
    //     },
    // }
}

async fn webhook_delete(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let webhook_id = r.match_info().query("id").parse().unwrap();

    let result = db::delete_webhook(
        webhook_id,
        &conn
    );
    
    match result {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::basic::BasicAuth;

// TODO: not sure the best way to authenticate from rails server
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

    let (sender, receiver) = channel::<WebhookMessage>();
    // TODO: move to new module
    actix::spawn(async move {        
        loop {
            let r = receiver.recv();
            let message = match r {
                Ok(m) => m,
                Err(e) => {
                    println!("Error receiving webhook message: {:?}", e);
                    continue;
                }
            };
            // let mut requests = Vec::new();
            for url in message.urls {
                println!("Webhook sending to {:?}", url);
                // let client = Client::default();
                // requests.push(
                //     client.post(url)
                //         .header("Content-Type", "application/json")
                //         .header("User-Agent", "Actix-web")
                //         .send_body(message.message.clone())
                //     );
            }
            // join_all(requests).await;
        }
    });

    HttpServer::new(move || {
        let pool = db::init_pool();
        let webhook_publisher_addr = webhook_publisher::WebhookPublisher::initialize(pool.clone(), sender.clone()).start();
        let publisher_addr = publisher::Publisher::initialize(
            pool.clone(),
            webhook_publisher_addr.clone(),
        ).start();
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .data(pool)
            .data(publisher_addr.clone())
            .service(web::resource("/").route(web::get().to(health_check)))
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            .service(web::resource("/message").route(web::post().to(message)))
            // .wrap(HttpAuthentication::basic(validator))
            .service(web::resource("/device_types")
                .route(web::post().to(device_types_post))
                .route(web::get().to(get_device_types)))
            .service(web::resource("/topics")
                .route(web::get().to(get_topics))
                .route(web::post().to(topics_post)))
            .service(web::resource("/webhooks")
                .route(web::get().to(get_webhooks))
                .route(web::post().to(webhooks_post)))
            .service(web::resource("/webhooks/{id}")
                .route(web::delete().to(webhook_delete)))
            .service(web::resource("/webhooks/{id}/topics")
                .route(web::get().to(get_webhook_topics)))
            .service(web::resource("/webhook_topics")
                .route(web::post().to(webhook_topics_post)))
            .service(web::resource("/webhook_topics/{id}")
                .route(web::delete().to(delete_webhook_topic)))
    })
    // TODO: add --release flag to binary such that it can
    // start on 0.0.0.0:8080 for release and 127.0.0.1:8080
    // for development
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
