use std::time::{Duration, Instant};
use actix::prelude::*;
use actix_web::{web};
use actix_web_actors::ws;
use serde_json::{Result as SerdeResult, Value};
use serde::{Deserialize};

use crate::db;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WebSocket {
    account_id: String, // The account associated with the connection
    device_id: String, // The unique device (not type) connected
    hb: Instant,
    pool: web::Data<db::DbPool>,
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
            pool
        }
    }

    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}


#[derive(Deserialize)]
struct DataType {
    component_id: String,
    data: Value,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let json_message: SerdeResult<DataType> = serde_json::from_str(&text);

                match json_message {
                    Ok(json) => {
                        println!("Successful message from {}", self.account_id);
                        println!("ID: {}", json.component_id);
                        println!("{}", json.data);
                        let conn = self.pool.get().expect("Failed to get a db connection");

                        db::insert_event(
                            &json.component_id,
                            &self.device_id,
                            json.data,
                            &conn
                        ).expect("Failed to perfrom insert");
                    }
                    Err(err) => {
                        println!("{:?}", err);
                    }
                }
                ctx.text(text);
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(_)) => ctx.stop(),
            _ => {
                println!("Bad formed data from {}", self.account_id);
            },
        }
    }
}