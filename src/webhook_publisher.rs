use actix::prelude::*;
use std::time::{Duration};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;

use crate::db;
use crate::db::DbPool;

use crate::publisher::{PublishMessage};

const WEBHOOK_UPDATE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct WebhookMessage {
    pub urls: HashSet<String>, 
    pub message: String,
}

pub struct WebhookPublisher {
    pool: DbPool,
    sender: Sender<WebhookMessage>,
    topics: HashMap<String, HashSet<String>>
}

impl Actor for WebhookPublisher {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        println!("Webhook publisher started");
        WebhookPublisher::refresh_webhooks(self);
        self.webhook_refresh_interval(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        println!("Webhook publisher stopped.");
    }
}

impl WebhookPublisher {
    pub fn initialize(pool: DbPool, sender: Sender<WebhookMessage>) -> WebhookPublisher {
        WebhookPublisher {
            pool,
            sender,
            topics: HashMap::new()
        }
    }

    fn refresh_webhooks(web: &mut WebhookPublisher) {
        match web.pool.get() {
            Ok(conn) => {
                let maybe_topics = db::get_all_webhook_topics(&conn);
                let topics = match maybe_topics {
                    Ok(t) => t,
                    Err(e) => {
                        println!("Error getting topics: {:?}", e);
                        return;
                    },
                };
                for item in topics {
                    match web.topics.get_mut(&item.0) {
                        Some(set) => {
                            set.insert(item.1);
                        },
                        None => {
                            let mut set = HashSet::new();
                            set.insert(item.1);
                            web.topics.insert(item.0.clone(), set);
                        }
                    }
                }
            }
            Err(e) => println!("Error getting db connection: {:?}", e),
        }
    }

    fn webhook_refresh_interval(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(WEBHOOK_UPDATE_INTERVAL, |act, _ctx| {
            WebhookPublisher::refresh_webhooks(act);
        });
    }
}

impl Handler<PublishMessage> for WebhookPublisher {
    type Result = ();

    fn handle(&mut self, msg: PublishMessage, _ctx: &mut Context<Self>) -> Self::Result {
        let mut webhooks: HashSet<String> = HashSet::new();
        // Find all the webhooks that should receive a message
        for topic in msg.message.topics.iter() {
            let topic_webhooks = match self.topics.get(topic) {
                Some(d) => d,
                None => continue,
            };
            // Iterate through all the devices of a topic
            // and insert into the hashset
            for url in topic_webhooks.iter() {
                webhooks.insert(url.to_owned());
            }
        }

        let message = serde_json::to_string(&msg.message);
        let serialized_message = match message {
            Ok(m) => m,
            Err(e) => {
                println!("Error serializing message: {:?}", e);
                return;
            },
        };
        let webhook_message = WebhookMessage {
            urls: webhooks,
            message: serialized_message,
        };
        println!("HIYA: {:?}", webhook_message);
        match self.sender.send(webhook_message) {
            Ok(_) => (),
            Err(e) => println!("Error sending out messages: {:?}", e),
        };
    }
}