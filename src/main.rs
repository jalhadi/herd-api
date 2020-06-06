#[macro_use]
extern crate diesel;
extern crate ctrlc;
extern crate ring;
extern crate data_encoding;
extern crate rand;
use std::any::Any;
use std::sync::Arc;
use std::collections::{HashSet};
use std::env;

use serde_json::Value;
use actix;
use actix::prelude::*;
use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::extractors::basic::BasicAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web::{middleware, web, App, error, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize};
use futures::executor;

mod auth;
mod db;
mod websocket;
mod rate_limiter;
mod publisher;
mod webhook_publisher;
mod utils;
mod logging;
mod pagination;
mod account;

pub mod schema;
pub mod models;

fn return_result<T: Any>(result: Result<T, diesel::result::Error>) -> Result<HttpResponse, Error> {
    match result {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

fn return_body<T: serde::Serialize>(data: T) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&data).unwrap()))
}

fn return_result_body<T: serde::Serialize>(result: Result<T, diesel::result::Error>) -> Result<HttpResponse, Error> {
    match result {
        Ok(res) => return_body(res),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// TODO: better error handling
async fn ws_index(
    auth: BasicAuth,
    pool: web::Data<db::DbPool>,
    r: HttpRequest,
    stream: web::Payload,
    publish: web::Data<Addr<publisher::Publisher>>,
    api_cipher_key: web::Data<ApiCipherKey>,
) -> Result<HttpResponse, Error> {
    // Validate websocket connection
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id = auth::authenticate_connection(auth, &api_cipher_key.0, &conn)?;

    let device_id: &str = r.headers().get("Device-Id").unwrap().to_str().unwrap();
    let device_type_id: &str = r.headers().get("Device-Type-Id").unwrap().to_str().unwrap();

    let relation_exists = db::device_type_relation_exists(
        &account_id,
        device_type_id,
        &conn,
    );

    match relation_exists {
        true => (),
        false => {
            return Ok(HttpResponse::Forbidden().finish());
        }
    }

    let account = account::get_account(&account_id, &conn).expect("Failed to get account.");

    db::create_device(
        device_id,
        device_type_id,
        &conn
    ).expect("Failed to create device");

    let res = ws::start(websocket::WebSocket::new(
        account_id,
        device_id.to_string(),
        device_type_id.to_string(),
        account.max_requests_per_minute,
        publish.get_ref().clone(),
        pool.clone(),
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

    let result = db::get_device_types(account_id, &conn);
    return_result_body(result)
}

async fn get_topics(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let result = db::get_topics(account_id, &conn);
    return_result_body(result)
}

#[derive(Debug, Deserialize)]
struct TopicsPost {
    name: String,
    description: Option<String>,
}

async fn topics_post(pool: web::Data<db::DbPool>, r: HttpRequest, body: web::Json<TopicsPost>) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let result = db::create_topic(
        &body.name,
        account_id,
        body.description.as_deref(),
        &conn
    );
    return_result_body(result)
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

    let result = db::create_webhook(
        account_id,
        &body.url,
        &conn
    );
    return_result_body(result)
}

async fn get_webhook_topics(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let webhook_id = r.match_info().query("id").parse().unwrap();

    let result = db::get_webhook_topics(webhook_id, &conn);
    return_result_body(result)
}

async fn delete_webhook_topic(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let id = r.match_info().query("id").parse().unwrap();

    let result = db::delete_webhook_topic(id, &conn);
    return_result(result)
}

#[derive(Debug, Deserialize)]
struct WebhookTopicsPost {
    webhook_id: i32,
    topic_ids: Vec<String>,
}

async fn webhook_topics_post(pool: web::Data<db::DbPool>, body: web::Json<WebhookTopicsPost>) -> Result<HttpResponse, Error> {
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
}

async fn webhook_delete(pool: web::Data<db::DbPool>, r: HttpRequest) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let webhook_id = r.match_info().query("id").parse().unwrap();

    let result = db::delete_webhook(
        webhook_id,
        &conn
    );
    return_result(result)
}

#[derive(Deserialize, Debug)]
struct LogQuery {
    page: Option<u32>,
    limit: Option<u32>
}

async fn get_logs(
    pool: web::Data<db::DbPool>,
    r: HttpRequest,
    query: web::Query<LogQuery>
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let result = logging::paginated_logs(
        account_id,
        query.page,
        query.limit,
        &conn
    );

    return_result_body(result)
}

async fn get_api_key(
    pool: web::Data<db::DbPool>,
    api_cipher_key: web::Data<ApiCipherKey>,
    r: HttpRequest,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let result = account::get_api_key(account_id, &api_cipher_key.0, &conn);
    return_result_body(result)
}

async fn create_account(
    pool: web::Data<db::DbPool>,
    api_cipher_key: web::Data<ApiCipherKey>,
    r: HttpRequest,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    match account::create_account(account_id, &api_cipher_key.0, &conn) {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => {
            Ok(HttpResponse::BadRequest().finish())
        },
    }
}

async fn get_account(
    pool: web::Data<db::DbPool>,
    r: HttpRequest,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("Failed to get a db connection");
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let result = account::get_account(account_id, &conn);
    return_result_body(result)
}

async fn get_account_activity(
    publish: web::Data<Addr<publisher::Publisher>>,
    r: HttpRequest,
) -> Result<HttpResponse, Error> {
    let account_id: &str = r.headers().get("Account-Id").unwrap().to_str().unwrap();

    let maybe_activity = publish.send(publisher::GetAccountActivity(account_id.to_owned())).await;
    match maybe_activity {
        Ok(some_activity) => match some_activity {
            Some(activity) => return_body(activity),
            None => return_body(HashSet::<publisher::Device>::new()),
        },
        Err(_) => Ok(HttpResponse::BadRequest().finish()),
    }
}

struct HmacKey(String);
struct ApiCipherKey(String);

async fn validator<'a>(
    req: ServiceRequest,
    _credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
    let hmac_key = req.app_data::<HmacKey>().unwrap().into_inner();

    let account_id: &str = req.headers().get("Account-Id").unwrap().to_str().unwrap();
    let time: &str = req.headers().get("Time").unwrap().to_str().unwrap();
    let path: &str = &req.uri().to_string();
    let hmac_signature: &str = req.headers().get("Herd-Webapp-Signature").unwrap().to_str().unwrap();

    let signature_good = auth::verify_hmac_sigature(
        path,
        account_id,
        time,
        hmac_signature,
        &hmac_key.0
    );

    match signature_good {
        true => Ok(req),
        false => Err(error::ErrorUnauthorized("Unauthorized"))
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    dotenv::dotenv().ok();

    // Comment to force rebuild
    let hmac_key = env::var("HMAC_KEY").expect("HMAC_KEY must be set");
    let api_cipher_key = env::var("API_CIPHER_KEY").expect("API_CIPHER_KEY must be set");

    let pool = db::init_pool();
    let webhook_publisher_addr = webhook_publisher::WebhookPublisher::initialize(pool.clone()).start();
    let publisher_addr = publisher::Publisher::initialize(
        pool.clone(),
        webhook_publisher_addr,
    ).start();

    let weak_publish_addr = publisher_addr.downgrade();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(pool.clone())
            .data(publisher_addr.clone())
            .data(HmacKey(hmac_key.clone()))
            .data(ApiCipherKey(api_cipher_key.clone()))
            .service(web::resource("/").route(web::get().to(health_check)))
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            .service(web::resource("/message").route(web::post().to(message)))
            .service(
                web::scope("/")
                /*
                    Not ideal, but currently using the HttpAuthentication
                    middlewares. They require that the authorization header
                    is present, so currently adding one, even though it's not
                    used. Lower priority, but ideally make own middleware
                    to then remove the unsued BearerAuth parameter in validator.
                */
                .wrap(HttpAuthentication::bearer(validator))
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
                .service(web::resource("/logs")
                    .route(web::get().to(get_logs)))
                .service(web::resource("/api_key")
                    .route(web::get().to(get_api_key)))
                .service(web::resource("/account")
                    .route(web::get().to(get_account))
                    .route(web::post().to(create_account)))
                .service(web::resource("/active_devices")
                    .route(web::get().to(get_account_activity)))
            )
    })
    // TODO: add --release flag to binary such that it can
    // start on 0.0.0.0:8080 for release and 127.0.0.1:8080
    // for development
    .bind("0.0.0.0:8080")?
    .run();

    let srv = server.clone();
    ctrlc::set_handler(move || {
        /*
            On shutdown, need to send messages to websockets
            to send a shutdown request, therefore allowing
            daemons to disconnect and then reconnect with
            new server
        */
        match weak_publish_addr.upgrade() {
            Some(addr) => addr.do_send(publisher::Shutdown()),
            None => (),
        }
        executor::block_on(srv.stop(true));
    }).expect("Error setting Ctrl-C handler");

    server.await
}
