use std::time::{Duration, Instant};
use actix::prelude::*;
use actix_web::{web};
use actix_web_actors::ws;
use serde_json::{Result as SerdeResult, Value};
use serde::{Deserialize};
use chrono::{NaiveDateTime};

use crate::db;
use crate::rate_limiter::RateLimit;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WebSocket {
    account_id: String, // The account associated with the connection
    device_id: String, // The unique device (not type) connected
    hb: Instant,
    pool: web::Data<db::DbPool>,
    rate_limit_struct: RateLimit,
    rate_limit: u64,
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl WebSocket {
    pub fn new(account_id: String, device_id: String, pool: web::Data<db::DbPool>) -> Self {
        Self {
            account_id,
            device_id,
            hb: Instant::now(),
            pool,
            rate_limit_struct: RateLimit::new(),
            // TODO: this value should come from the api server
            // and be update every so often. If someone
            // updates their account to allow higher limit,
            // it should be reflected without having to restart
            // connection.
            rate_limit: 100,
        }
    }

    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }

            // TODO: maybe call webapp here to update
            // certain account metrics every few
            // heartbeats

            ctx.ping(b"");
        });
    }
}

#[derive(Deserialize)]
struct Data {
    component_id: String,
    json: Value,
}

#[derive(Deserialize)]
struct Event {
    seconds_since_unix: u64,
    nano_seconds: u32,
    data: Data
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        println!("{:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                // TODO: currently, this is a rate limit on
                // the device level, not the account level.
                self.rate_limit_struct.update_request_count();
                if self.rate_limit_struct.requests > self.rate_limit {
                    return;
                }

                let maybe_event: SerdeResult<Event> = serde_json::from_str(&text);

                match maybe_event {
                    Ok(event) => {
                        println!("Successful message from {}", self.account_id);
                        println!("ID: {}", event.data.component_id);
                        println!("{}", event.data.component_id);
                        let conn = self.pool.get().expect("Failed to get a db connection");

                        let event_timestamp = NaiveDateTime::from_timestamp(
                            event.seconds_since_unix as i64,
                            event.nano_seconds
                        );

                        let insert_result = db::insert_event(
                            &event.data.component_id,
                            &self.device_id,
                            event.data.json,
                            event_timestamp,
                            &conn
                        );

                        match insert_result {
                            Ok(_) => println!("Insert successful."),
                            Err(_) => println!("Insert failed."),
                        }
                    }
                    Err(err) => {
                        // TODO: return a useful error message
                        println!("JSON parse error: {:?}", err);
                    }
                }
                ctx.text(text);
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(_)) => ctx.stop(),
            _ => {
                // TODO: return a useful error message
                println!("Bad formed data from {}", self.account_id);
            },
        }
    }
}