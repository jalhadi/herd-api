use actix::prelude::*;
use std::time::{Duration};
use std::collections::{HashMap, HashSet};
use serde::{Serialize};
use std::net::SocketAddr;
use serde_json::json;
use serde_json::Value;

use crate::websocket::Message;

use crate::websocket::{WebSocket};
use crate::webhook_publisher::{WebhookPublisher};

use crate::db;
use crate::db::DbPool;
use crate::logging;
use crate::account;

const TOPIC_RELATIONS_UPDATE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Serialize, Clone)]
pub enum Sender {
    Address(Option<SocketAddr>),
    Device {
        device_id: String,
        device_type_id: String,
    }
}

#[derive(Message, Serialize, Clone)]
#[rtype(result = "()")]
pub struct PublishMessage {
    pub sender: Sender,
    pub account_id: String,
    pub message: Message,
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct RegisterTopics {
    pub account_id: String,
    pub device_id: String,
    pub topics: Vec<String>,
}

#[derive(Message)]
#[rtype(result = "Result<(), &'static str>")]
pub struct Connect {
    pub account_id: String,
    pub device_type_id: String,
    pub device_id: String,
    pub addr: Addr<WebSocket>
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect{
    pub account_id: String,
    pub device_type_id: String,
    pub device_id: String,
}

#[derive(Hash, Eq, PartialEq, Clone, Serialize)]
pub struct Device {
    device_id: String,
    // Including device_type_id is not strictly necessary,
    // adding it here just saves a query in the future,
    // but maybe that should be done now?
    device_type_id: String,
}

#[derive(Clone)]
pub struct Account {
    // Ideally just store a hashset of device_ids
    // and query the device_type_id
    pub devices: HashSet<Device>,
    pub max_connections: usize,
}

// When a server is being taken down and another
// put up, need to close the websocket connections
// and tell them to reopen
#[derive(Message)]
#[rtype(result = "()")]
pub struct Shutdown();

#[derive(Message)]
#[rtype(result = "Option<HashSet<Device>>")]
pub struct GetAccountActivity(pub String);

pub struct Publisher {
    pool: DbPool,
    webhook_publisher: Addr<WebhookPublisher>,
    // device_ids to WebSocket address
    sessions: HashMap<String, Addr<WebSocket>>,
    // topic_id to HashSet of device_ids
    topics: HashMap<String, HashSet<String>>,
    // account_id to HashSet of topic_ids
    topic_relations: HashMap<String, HashSet<String>>,
    // active account connections
    // account_id -> Account
    accounts: HashMap<String, Account>,
}

impl Publisher {
    pub fn initialize(pool: DbPool, webhook_publisher: Addr<WebhookPublisher>) -> Publisher {
        Publisher {
            pool,
            webhook_publisher,
            sessions: HashMap::new(),
            topics: HashMap::new(),
            topic_relations: HashMap::new(),
            accounts: HashMap::new(),
        }
    }

    fn topic_relations_refresh(publisher: &mut Publisher) {
        match publisher.pool.get() {
            Ok(conn) => {
                let maybe_relations = db::get_all_topic_relations(&conn);
                let relations = match maybe_relations {
                    Ok(r) => r,
                    Err(_) => {
                        return;
                    }
                };
                for item in relations {
                    match publisher.topic_relations.get_mut(&item.account_id) {
                        Some(set) => {
                            set.insert(item.id);
                        },
                        None => {
                            let mut set = HashSet::new();
                            set.insert(item.id);
                            publisher.topic_relations.insert(item.account_id.clone(), set);
                        }
                    }
                }
            },
            Err(e) => println!("Error getting database connection: {:?}", e),
        }
    }

    fn topic_relations_refresh_interval(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(TOPIC_RELATIONS_UPDATE_INTERVAL, |act, _ctx| {
            Publisher::topic_relations_refresh(act);
        });
    }
}

impl Actor for Publisher {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        println!("Publisher started. Waiting for connections.");
        Publisher::topic_relations_refresh(self);
        self.topic_relations_refresh_interval(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        println!("Publisher stopped.");
    }
}

impl Handler<Connect> for Publisher {
    type Result = Result<(), &'static str>;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        match self.accounts.get_mut(&msg.account_id) {
            Some(account) => {
                if account.devices.len() >= account.max_connections {
                    logging::log(
                        &msg.account_id,
                        logging::LogLevel::Error,
                        json!({
                            "device_id": msg.device_id,
                            "device_type_id": msg.device_type_id,
                            "message": "max connections error"
                        }),
                        &self.pool
                    );
                    return Err("Exceeded number of connections");
                }
                account.devices.insert(Device {
                    device_id: msg.device_id.clone(),
                    device_type_id: msg.device_type_id.clone(),
                });
            },
            None => {
                let conn = self.pool.get();
                let conn = match conn {
                    Ok(c) => c,
                    Err(_) => {
                        logging::log(
                            &msg.account_id,
                            logging::LogLevel::Error,
                            json!({
                                "device_id": msg.device_id,
                                "device_type_id": msg.device_type_id,
                                "message": "connection error"
                            }),
                            &self.pool
                        );
                        return Err("Unable to get connection");
                    },
                };

                let account = account::get_account(
                    &msg.account_id,
                    &conn,
                );
                let account = match account {
                    Ok(a) => a,
                    Err(_) => {
                        logging::log(
                            &msg.account_id,
                            logging::LogLevel::Error,
                            json!({
                                "device_id": msg.device_id,
                                "device_type_id": msg.device_type_id,
                                "message": "connection error"
                            }),
                            &self.pool
                        );
                        return Err("Unable to fetch account");
                    },
                };

                let mut devices = HashSet::new();
                devices.insert(Device {
                    device_id: msg.device_id.clone(),
                    device_type_id: msg.device_type_id.clone(),
                });
                let new_account = Account {
                    devices,
                    max_connections: account.max_connections as usize,
                };
                self.accounts.insert(
                    msg.account_id.clone(),
                    new_account,
                );
            },
        }
        self.sessions.insert(msg.device_id.clone(), msg.addr);

        logging::log(
            &msg.account_id,
            logging::LogLevel::Info,
            json!({
                "device_id": msg.device_id,
                "device_type_id": msg.device_type_id,
                "message": "connected"
            }),
            &self.pool
        );

        Ok(())
    }
}

impl Handler<RegisterTopics> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: RegisterTopics, _: &mut Context<Self>) -> Self::Result {
        let conn = self.pool.get().expect("Failed to get a db connection");

        for topic in msg.topics {
            // Check that an account can receive a topic
            if !db::topic_relation_exists(&msg.account_id, &topic, &conn) {
                continue;
            }
            match self.topics.get_mut(&topic) {
                // Insert device_id into existing HashSet if it exists
                Some(v) => {
                    v.insert(msg.device_id.clone());
                },
                // Otherwise create a new HashSet, push the device_id
                // and insert the HashSet into the hash
                None => {
                    let mut new_topic = HashSet::new();
                    new_topic.insert(msg.device_id.clone());
                    self.topics.insert(topic, new_topic);
                },
            }
        }
    }
}

impl Handler<Disconnect> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        logging::log(
            &msg.account_id,
            logging::LogLevel::Info,
            json!({
                "device_id": msg.device_id,
                "device_type_id": msg.device_type_id,
                "message": "disconnected"
            }),
            &self.pool
        );
        match self.accounts.get_mut(&msg.account_id) {
            Some(account) => {
                account.devices.remove(&Device {
                    device_id: msg.device_id.clone(),
                    device_type_id: msg.device_type_id.clone()
                });
            },
            None => {
                eprintln!(
                    "{} closing for account {}, but no account struct in publisher.",
                    &msg.device_id,
                    &msg.account_id,
                );
            }
        }

        self.sessions.remove(&msg.device_id);
    }
}

impl Handler<Shutdown> for Publisher {
    type Result = ();

    fn handle(&mut self, _msg: Shutdown, _: &mut Context<Self>) -> Self::Result {
        println!("Publisher sending shutdown...");
        for (device_id, addr) in self.sessions.iter() {
            // TODO: add logging to record when shutdowns
            // for deployment happen
            println!("device_id: {:?}", device_id);
            addr.do_send(Shutdown());
        }
    }
}

impl Handler<PublishMessage> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: PublishMessage, _ctx: &mut Context<Self>) -> Self::Result {
        let mut devices: HashSet<String> = HashSet::new();
        // Find all the actors that should receive a message
        for topic in msg.message.topics.iter() {
            // Only send to topics that an account has
            // a relation with
            match self.topic_relations.get(&msg.account_id) {
                Some(topics) => {
                    match topics.get(topic) {
                        Some(_) => (),
                        None => continue,
                    }
                },
                None => continue,
            }

            let topic_devices = match self.topics.get(topic) {
                Some(d) => d,
                None => continue,
            };
            // Iterate through all the devices of a topic
            // and insert into the hashset
            for device in topic_devices.iter() {
                if let Sender::Device { device_id, .. } = &msg.sender {
                    if device == device_id {
                        continue;
                    }
                }
                devices.insert(device.to_owned());
            }
        }
        // Publish message to webhook only if the
        // sender is a device (not from outside post)
        if let Sender::Device { .. } = &msg.sender {
            self.webhook_publisher.do_send(msg.clone());
        }

        let sender_json: Value = match msg.sender.clone() {
            Sender::Device { device_id, device_type_id} => json!({
                    "device_id": device_id,
                    "device_type_id": device_type_id
                }),
            Sender::Address(maybe_address) =>
                match maybe_address {
                    Some(address) => json!({
                        "ip": address.ip()
                    }),
                    None => Value::Null
                },
        };

        logging::log(
            &msg.account_id,
            logging::LogLevel::Info,
            json!({
                "sender": sender_json,
                "data": msg.message, 
                "message": "Message received"
            }),
            &self.pool
        );

        // TODO: need to check if a message is allowed
        // to send to a specific topic

        // Publish message to other devices
        for device in devices.iter() {
            let addr = match self.sessions.get(device) {
                Some(s) => s,
                None => continue,
            };
            addr.do_send(msg.clone());
        }
    }
}

impl Handler<GetAccountActivity> for Publisher {
    type Result = Option<HashSet<Device>>;

    fn handle(&mut self, msg: GetAccountActivity, _ctx: &mut Context<Self>) -> Self::Result {
        match self.accounts.get(&msg.0) {
            Some(account) => Some(account.devices.clone()),
            None => None,
        }
    }
}
