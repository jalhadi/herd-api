use std::time::{Duration, Instant};
use actix::prelude::*;
use actix_web_actors::ws;
use serde_json::{Result as SerdeResult, Value};
use serde::{Deserialize, Serialize};

use crate::publisher;
use crate::rate_limiter::RateLimit;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WebSocket {
    account_id: String, // The account associated with the connection
    device_id: String, // The unique device (not type) connected
    device_type_id: String,
    hb: Instant,
    publisher: Addr<publisher::Publisher>,
    subscribed_topics: Vec<String>,
    rate_limit_struct: RateLimit,
    rate_limit: u64,
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.publisher
            .send(publisher::Connect {
                account_id: self.account_id.clone(),
                device_id: self.device_id.clone(),
                device_type_id: self.device_type_id.clone(),
                addr,
            })
            // TODO: no clue what the rest of this function does
            // look into it
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_) => (),
                    // something is wrong with the websocket
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl WebSocket {
    pub fn new(
        account_id: String,
        device_id: String,
        device_type_id: String,
        publisher: Addr<publisher::Publisher>,
    ) -> Self {
        Self {
            account_id,
            device_id,
            device_type_id,
            hb: Instant::now(),
            publisher,
            rate_limit_struct: RateLimit::new(),
            subscribed_topics: Vec::new(),
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

#[derive(Serialize, Debug, Clone)]
pub struct Message {
    seconds_since_unix: u64,
    nano_seconds: u32,
    pub topics: Vec<String>,
    data: Value
}

#[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(untagged)]
pub enum Event {
    Message {
        seconds_since_unix: u64,
        nano_seconds: u32,
        topics: Vec<String>,
        data: Value
    },
    Register {
        topics: Vec<String>
    }
}

impl Handler<publisher::PublishMessage> for WebSocket {
    type Result = ();

    fn handle(&mut self, msg: publisher::PublishMessage, ctx: &mut ws::WebsocketContext<Self>) -> Self::Result {
        match serde_json::to_string(&msg) {
            Ok(m) => ctx.text(m),
            Err(e) => println!("Error serializing message: {:?}", e),
        };
        ()
    }
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

                println!("Received message: {:?}", text);

                let maybe_event: SerdeResult<Event> = serde_json::from_str(&text);

                match maybe_event {
                    Ok(event) => {
                        println!("Got event: {:?}", event);
                        
                        match event {
                            Event::Message {
                                seconds_since_unix,
                                nano_seconds,
                                topics,
                                data,
                            } => {
                                self.publisher
                                    .do_send(publisher::PublishMessage {
                                        sender_device_type_id: self.device_type_id.clone(),
                                        message: Message {
                                            seconds_since_unix,
                                            nano_seconds,
                                            topics,
                                            data
                                        }
                                    });
                            },
                            Event::Register { topics } => {
                                println!("Register");
                                self.publisher
                                    .do_send(publisher::RegisterTopics {
                                        account_id: self.account_id.clone(),
                                        device_id: self.device_id.clone(),
                                        topics,
                                    });
                            },
                        }
                    },
                    Err(err) => {
                        // TODO: return a useful error message
                        println!("JSON parse error: {:?}", err);
                    }
                }
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(_)) => {
                self.publisher
                    .do_send(publisher::Disconnect(self.device_type_id.clone()));
                ctx.stop()
            },
            _ => {
                // TODO: return a useful error message
                println!("Bad formed data from {}", self.account_id);
            },
        }
    }
}