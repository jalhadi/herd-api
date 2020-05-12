use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize};

use crate::websocket::Message;

use crate::websocket::{WebSocket};
use crate::webhook_publisher::{WebhookPublisher};

use crate::db;
use crate::db::DbPool;

#[derive(Message, Serialize, Clone)]
#[rtype(result = "()")]
pub struct PublishMessage {
    pub sender_device_type_id: String,
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
#[rtype(result = "()")]
pub struct Connect {
    pub account_id: String,
    pub device_type_id: String,
    pub device_id: String,
    pub addr: Addr<WebSocket>
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect(pub String);

pub struct Publisher {
    pool: DbPool,
    webhook_publisher: Addr<WebhookPublisher>,
    sessions: HashMap<String, Addr<WebSocket>>,
    topics: HashMap<String, Vec<String>>,
}

impl Publisher {
    pub fn initialize(pool: DbPool, webhook_publisher: Addr<WebhookPublisher>) -> Publisher {
        Publisher {
            pool,
            webhook_publisher,
            sessions: HashMap::new(),
            topics: HashMap::new(),
        }
    }
}

impl Actor for Publisher {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        println!("Publisher started. Waiting for connections.");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        println!("Publisher stopped.");
    }
}

impl Handler<Connect> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Device connected.");

        self.sessions.insert(msg.device_id.clone(), msg.addr);
    }
}

impl Handler<RegisterTopics> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: RegisterTopics, _: &mut Context<Self>) -> Self::Result {
        let conn = self.pool.get().expect("Failed to get a db connection");

        for topic in msg.topics {
            if !db::topic_relation_exists(&msg.account_id, &topic, &conn) {
                continue;
            }
            println!("Attempting to register topic: {:?}", topic);
            match self.topics.get_mut(&topic) {
                // Insert device_id into existing vector if it exists
                Some(v) => v.push(msg.device_id.clone()),
                // Otherwise create a new vector, push the device_id
                // and insert the vector into the hash
                None => {
                    let mut new_topic = Vec::new();
                    new_topic.push(msg.device_id.clone());
                    self.topics.insert(topic, new_topic);
                },
            }
        }
        println!("Topics: {:?}", self.topics);
    }
}

impl Handler<Disconnect> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        println!("{:?} is disconnecting.", msg.0);

        self.sessions.remove(&msg.0);
    }
}

impl Handler<PublishMessage> for Publisher {
    type Result = ();

    fn handle(&mut self, msg: PublishMessage, _ctx: &mut Context<Self>) -> Self::Result {
        println!("Event received: {:?}", msg.message);

        let mut devices: HashSet<String> = HashSet::new();
        // Find all the actors that should receive a message
        for topic in msg.message.topics.iter() {
            let topic_devices = match self.topics.get(topic) {
                Some(d) => d,
                None => continue,
            };
            // Iterate through all the devices of a topic
            // and insert into the hashset
            for device in topic_devices.iter() {
                if device == &msg.sender_device_type_id {
                    continue;
                }
                devices.insert(device.to_owned());
            }
        }
        // Publish message to webhook
        self.webhook_publisher.do_send(msg.clone());

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