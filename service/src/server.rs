use actix::prelude::*;
use mongodb::Database;
use std::collections::HashMap;
use std::convert::TryInto;

use crate::graphql::simple_broker::SimpleBroker;
use crate::graphql::{
    CommandStatus, CommandType, OpenerCommandResult, OpenerConnectionChanged, OpenerError,
};
use crate::persistence::barrier_model::get_barrier_model_by_id;
use crate::persistence::opener::{
    get_opener_by_sn, set_command_to_opener_with_model, set_error_to_opener, update_opener,
    OpenerErrorEntity, UpdateOpenerEntity,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub(crate) mod command;
pub(crate) mod message;

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
                Err(e) => {
                    log::error!("Failed to find barrier model: {}", e);
                    None
                }
                Ok(m) => m,
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
            barrier_model_id: model.and_then(|m| m.id.map(|i| i.to_string())),
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

    async fn handle_set_message(
        db: &Database,
        command: &command::SetCommand,
    ) -> Result<(), String> {
        log::info!("Process set from opener {}", command.serial_number);

        let opener = match get_opener_by_sn(db, &command.serial_number).await {
            Err(e) => {
                log::error!(
                    "Failed to found opener {}: {}",
                    command.serial_number,
                    e.to_string()
                );
                return Err(e.to_string());
            }
            Ok(opener) => match opener {
                Some(opener) => {
                    log::info!("Opener {} founded", command.serial_number);
                    opener
                }
                None => {
                    log::error!("Opener {} not found", command.serial_number,);
                    return Err("Opener not found".to_string());
                }
            },
        };

        match set_command_to_opener_with_model(
            db,
            &command.serial_number,
            "SUCCESS",
            &command.arguments.barrier_model,
        )
        .await
        {
            Err(e) => {
                log::error!(
                    "Failed to update opener {}: {}",
                    command.serial_number,
                    e.to_string()
                );
                return Err(e.to_string());
            }
            Ok(_) => {
                log::info!("Opener {} updated", command.serial_number);
            }
        };

        log::info!("Publish set command result");

        SimpleBroker::publish(OpenerCommandResult {
            serial_number: command.serial_number.clone(),
            command_type: CommandType::Set,
            command_status: CommandStatus::Success,
            error: None,
            user_id: opener.user_id.map(|user_id| user_id.to_string()),
        });

        Ok(())
    }

    async fn handle_error_message(
        db: &Database,
        msg: &message::Error,
        command_type: String,
    ) -> Result<(), String> {
        log::info!("Process error from opener {}", msg.serial_number);

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

        match set_error_to_opener(
            db,
            &msg.serial_number,
            "FAILED",
            OpenerErrorEntity {
                serial_number: msg.serial_number.clone(),
                code: msg.code,
                description: msg.description.clone(),
                details: msg.details.clone(),
            },
        )
        .await
        {
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

        log::info!("Publish error result");

        let command_type: CommandType = command_type.as_str().try_into()?;

        SimpleBroker::publish(OpenerCommandResult {
            serial_number: msg.serial_number.clone(),
            command_type,
            command_status: CommandStatus::Failed,
            error: Some(OpenerError {
                serial_number: msg.serial_number.clone(),
                code: msg.code,
                description: msg.description.clone(),
                details: msg.details.clone(),
            }),
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
            return Box::pin(async { });
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
            },
        };

        self.commands
            .insert(msg.serial_number, command::Command::Set(command.clone()));

        let fut = async move {
            if let Err(e) = addr.send(command).await {
                log::error!("Failed to send set command to controller: {}", e);
            }
        };

        Box::pin(fut)
    }
}

impl Handler<message::Set> for OpenerServer {
    type Result = ResponseFuture<Result<(), String>>;

    fn handle(&mut self, msg: message::Set, _: &mut Context<Self>) -> Self::Result {
        log::info!("Opener {} sent set message", msg.serial_number);

        let db: Database = self.db.clone();

        let command = self.commands.get(&msg.serial_number);

        if command.is_none() {
            log::error!("Set command for {} not found", msg.serial_number);
            return Box::pin(async { Ok(()) });
        }

        let command = command.unwrap();

        let command = match command {
            command::Command::Set(c) => c,
            _ => {
                log::error!(
                    "Wrong command instead of set command for {} found",
                    msg.serial_number
                );
                return Box::pin(async { Ok(()) });
            }
        };

        let command = command.clone();

        let fut = async move { OpenerServer::handle_set_message(&db, &command).await };

        Box::pin(fut)
    }
}

impl Handler<message::Error> for OpenerServer {
    type Result = ResponseFuture<Result<(), String>>;

    fn handle(&mut self, msg: message::Error, _: &mut Context<Self>) -> Self::Result {
        log::info!("Opener {} sent error message", msg.serial_number);

        let db: Database = self.db.clone();
        let m = msg.clone();

        let command = self.commands.get(&msg.serial_number);

        if command.is_none() {
            log::error!("Set command for {} not found", msg.serial_number);
            return Box::pin(async { Ok(()) });
        }

        let command = command.unwrap();

        let command = match command {
            command::Command::Set(c) => c.clone().command,
            _ => {
                log::error!("Wrong command for {} found", msg.serial_number);
                return Box::pin(async { Ok(()) });
            }
        };

        let fut = async move { OpenerServer::handle_error_message(&db, &m, command).await };

        Box::pin(fut)
    }
}
