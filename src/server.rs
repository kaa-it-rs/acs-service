use actix::prelude::*;
use mongodb::Database;
use std::collections::HashMap;

use crate::graphql::simple_broker::SimpleBroker;
use crate::graphql::OpenerConnectionChanged;
use crate::persistence::opener::{get_opener_by_sn, update_opener, UpdateOpenerEntity};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub(crate) mod dsl;

pub struct OpenerServer {
    sessions: HashMap<String, Recipient<dsl::Message>>,
    count: Arc<AtomicUsize>,
    db: Database,
}

impl OpenerServer {
    pub fn new(count: Arc<AtomicUsize>, db: Database) -> Self {
        OpenerServer {
            sessions: HashMap::new(),
            count,
            db,
        }
    }

    async fn handle_connect(db: &Database, msg: &dsl::Connect) -> Result<(), String> {
        log::info!("Search opener {}", msg.serial_number);

        let opener = match get_opener_by_sn(db, &msg.serial_number).await {
            Err(e) => {
                log::error!(
                    "Failed to found opener {}: {}",
                    msg.serial_number,
                    e.to_string()
                );
                return Err(e.to_string());
            }
            Ok(opener) => match opener {
                Some(opener) => {
                    log::info!("Opener {} founded", msg.serial_number);
                    opener
                }
                None => {
                    log::error!("Opener {} not found", msg.serial_number,);
                    return Err("Opener not found".to_string());
                }
            },
        };

        let new_opener_entity = UpdateOpenerEntity {
            user_id: None,
            alias: None,
            description: None,
            lat: None,
            lng: None,
            login: None,
            password: None,
            connected: Some(true),
            nonce: Some(msg.nonce.clone()),
            version: Some(msg.version.clone()),
        };

        match update_opener(db, &msg.serial_number, &new_opener_entity).await {
            Err(e) => {
                log::error!(
                    "Failed to update opener {}: {}",
                    msg.serial_number,
                    e.to_string()
                );
                return Err(e.to_string());
            }
            Ok(_) => {
                log::info!("Opener {} updated", msg.serial_number);
            }
        };

        log::info!("Publish connected");

        SimpleBroker::publish(OpenerConnectionChanged {
            serial_number: msg.serial_number.clone(),
            connected: true,
            user_id: match opener.user_id {
                Some(user_id) => Some(user_id.to_string()),
                None => None,
            },
        });

        Ok(())
    }

    async fn handle_disconnect(db: &Database, msg: &dsl::Disconnect) -> Result<(), String> {
        log::info!("Search opener {}", msg.id);

        let opener = match get_opener_by_sn(db, &msg.id).await {
            Err(e) => {
                log::error!("Failed to found opener {}: {}", msg.id, e.to_string());
                return Err(e.to_string());
            }
            Ok(opener) => match opener {
                Some(opener) => {
                    log::info!("Opener {} founded", msg.id);
                    opener
                }
                None => {
                    log::error!("Opener {} not found", msg.id,);
                    return Err("Opener not found".to_string());
                }
            },
        };

        let new_opener_entity = UpdateOpenerEntity {
            user_id: None,
            alias: None,
            description: None,
            lat: None,
            lng: None,
            login: None,
            password: None,
            connected: Some(false),
            nonce: None,
            version: None,
        };

        match update_opener(db, &msg.id, &new_opener_entity).await {
            Err(e) => {
                log::error!("Failed to update opener {}: {}", msg.id, e.to_string());
                return Err(e.to_string());
            }
            Ok(_) => {
                log::info!("Opener {} updated", msg.id);
            }
        };

        SimpleBroker::publish(OpenerConnectionChanged {
            serial_number: msg.id.clone(),
            connected: false,
            user_id: match opener.user_id {
                Some(user_id) => Some(user_id.to_string()),
                None => None,
            },
        });

        Ok(())
    }
}

impl Actor for OpenerServer {
    type Context = Context<Self>;
}

impl Handler<dsl::Connect> for OpenerServer {
    type Result = ResponseActFuture<Self, Result<String, String>>;

    fn handle(&mut self, msg: dsl::Connect, _: &mut Context<Self>) -> Self::Result {
        log::info!("Opener {} connected", msg.serial_number);

        let db: Database = self.db.clone();
        let m = msg.clone();

        let fut = async move { OpenerServer::handle_connect(&db, &m).await };

        let wrapped_future = actix::fut::wrap_future::<_, Self>(fut);

        let res = wrapped_future.map(|result, actor, _ctx| match result {
            Ok(_) => {
                actor.sessions.insert(msg.serial_number.clone(), msg.addr);
                actor.count.fetch_add(1, Ordering::SeqCst);
                Ok(msg.serial_number)
            }
            Err(e) => Err(e),
        });

        Box::pin(res)
    }
}

impl Handler<dsl::Disconnect> for OpenerServer {
    type Result = ResponseFuture<Result<(), String>>;

    fn handle(&mut self, msg: dsl::Disconnect, _: &mut Context<Self>) -> Self::Result {
        if self.sessions.get(&msg.id).is_none() {
            return Box::pin(async { Ok(()) });
        } else {
            self.sessions.remove(&msg.id);
            self.count.fetch_sub(1, Ordering::SeqCst);
        }

        log::info!("Opener {} disconnected", msg.id);

        let db: Database = self.db.clone();
        let m = msg.clone();

        let fut = async move { OpenerServer::handle_disconnect(&db, &m).await };

        Box::pin(fut)
    }
}
