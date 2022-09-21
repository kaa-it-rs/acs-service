use actix::prelude::*;
use crate::server::command;

#[derive(Message, Clone)]
#[rtype(result = "Result<String, String>")]
pub struct Connect {
    pub addr: Recipient<command::SetCommand>,
    pub serial_number: String,
    pub version: String,
    pub nonce: String,
    pub barrier_model: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct Disconnect {
    pub id: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct Set {
  pub serial_number: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct Error {
  pub serial_number: String,
  pub code: u32,
  pub description: String,
  pub details: Option<String>,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SetCommand {
    pub login: String,
    pub password: String,
    pub nonce: String,
    pub serial_number: String,
    pub barrier_model: String,
    pub barrier_algorithm: String,
}