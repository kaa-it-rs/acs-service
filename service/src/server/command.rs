use actix::prelude::*;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct SetCommandArgs {
    pub barrier_model: String,
    pub barrier_algorithm: String,
}

#[derive(Message, Serialize, Clone)]
#[rtype(result = "()")]
pub struct SetCommand {
    pub serial_number: String,
    pub command: String,
    pub authorization: String,
    pub arguments: SetCommandArgs,
}

pub enum Command {
    Set(SetCommand),
    #[allow(unused)]
    Other
}