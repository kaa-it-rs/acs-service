use actix::prelude::*;
use mongodb::Database;
use std::collections::HashMap;

use crate::graphql::simple_broker::SimpleBroker;
use crate::graphql::OpenerConnectionChanged;
use crate::persistence::barrier_model::get_barrier_model_by_id;
use crate::persistence::opener::{get_opener_by_sn, update_opener, UpdateOpenerEntity};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub(crate) mod message;
pub(crate) mod command;

pub struct OpenerServer {
    sessions: HashMap<String, Recipient<command::SetCommand>>,
    commands: HashMap<String, command::Command>,
    count: Arc<AtomicUsize>,
    db: Database,
}

impl OpenerServer {
    pub fn new(count: Arc<AtomicUsize>, db: Database) -> Self {
        OpenerServer {
            sessions: HashMap::new(),
            commands: HashMap::new(),
            count,
            db,
        }
    }

    async fn handle_connect(db: &Database, msg: &message::Connect) -> Result<(), String> {
        log::info!("Process hello from opener {}", msg.serial_number);

        let model = if !msg.barrier_model.is_empty() {
            match get_barrier_model_by_id(db, &msg.barrier_model).await {
                Err(e) => return Err(e.to_string()),
                Ok(m) => m
            }
        } else {
            None
        };

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
            barrier_model_id: model.map(|m| m.id.map(|i| i.to_string())).flatten(),
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
            user_id: opener.user_id.map(|user_id| user_id.to_string()),
        });

        Ok(())
    }

    async fn handle_disconnect(db: &Database, msg: &message::Disconnect) -> Result<(), String> {
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
            barrier_model_id: None,
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
            user_id: opener.user_id.map(|user_id| user_id.to_string()),
        });

        Ok(())
    }
}

impl Actor for OpenerServer {
    type Context = Context<Self>;
}

impl Handler<message::Connect> for OpenerServer {
    type Result = ResponseActFuture<Self, Result<String, String>>;

    fn handle(&mut self, msg: message::Connect, _: &mut Context<Self>) -> Self::Result {
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

impl Handler<message::Disconnect> for OpenerServer {
    type Result = ResponseFuture<Result<(), String>>;

    fn handle(&mut self, msg: message::Disconnect, _: &mut Context<Self>) -> Self::Result {
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

impl Handler<message::SetCommand> for OpenerServer {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: message::SetCommand, _: &mut Context<Self>) -> Self::Result {
        let addr = self.sessions.get(&msg.serial_number);
        if addr.is_none() {
            return Box::pin(async { () });
        }

        let addr = addr.unwrap().clone();

        log::info!("Send set params command to opener {}", msg.serial_number);

        let credentials = format!("{}{}{}", msg.login, msg.password, msg.nonce);

        let hash = sha256::digest(credentials);

        let command = command::SetCommand {
            serial_number: msg.serial_number.clone(),
            command: "SET".to_string(),
            authorization: hash,
            arguments: command::SetCommandArgs {
                barrier_model: msg.barrier_model,
                barrier_algorithm: msg.barrier_algorithm,
            }
        };

        self.commands.insert(msg.serial_number, command::Command::Set(command.clone()));

        let fut = async move {
            if let Err(e) = addr.send(command).await {
                log::error!("Failed to send set command to controller: {}", e);
            }
        };

        Box::pin(fut)
    }
}

// impl Handler<message::Set> for OpenerServer {
//   type Result = ResponseActFuture<Self, Result<(), String>>;
//
//   fn handle(&mut self, msg: message::Set, _: &mut Context<Self>) -> Self::Result {
//       log::info!("Opener {} sent set message", msg.serial_number);
//
//       let db: Database = self.db.clone();
//       let m = msg.clone();
//
//       let fut = async move { OpenerServer::handle_connect(&db, &m).await };
//
//       let wrapped_future = actix::fut::wrap_future::<_, Self>(fut);
//
//       let res = wrapped_future.map(|result, actor, _ctx| match result {
//           Ok(_) => {
//               actor.sessions.insert(msg.serial_number.clone(), msg.addr);
//               actor.count.fetch_add(1, Ordering::SeqCst);
//               Ok(msg.serial_number)
//           }
//           Err(e) => Err(e),
//       });
//
//       Box::pin(res)
//   }
// }
//
// impl Handler<message::Error> for OpenerServer {
//   type Result = ResponseActFuture<Self, Result<(), String>>;
//
//   fn handle(&mut self, msg: message::Error, _: &mut Context<Self>) -> Self::Result {
//       log::info!("Opener {} connected", msg.serial_number);
//
//       let db: Database = self.db.clone();
//       let m = msg.clone();
//
//       let fut = async move { OpenerServer::handle_connect(&db, &m).await };
//
//       let wrapped_future = actix::fut::wrap_future::<_, Self>(fut);
//
//       let res = wrapped_future.map(|result, actor, _ctx| match result {
//           Ok(_) => {
//               actor.sessions.insert(msg.serial_number.clone(), msg.addr);
//               actor.count.fetch_add(1, Ordering::SeqCst);
//               Ok(msg.serial_number)
//           }
//           Err(e) => Err(e),
//       });
//
//       Box::pin(res)
//   }
//}
