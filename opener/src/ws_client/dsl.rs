use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HELLO_TYPE: &str = "HELLO";
pub const SET_TYPE: &str = "SET";
pub const ERROR_TYPE: &str = "ERROR";

#[derive(Serialize, Deserialize)]
pub struct HelloData {
    pub serial_number: String,
    pub version: String,
    pub nonce: String,
    pub barrier_model: String,
}

#[derive(Serialize, Deserialize)]
pub struct Hello {
    #[serde(rename = "type")]
    pub message_type: String,

    pub data: HelloData,
}

#[derive(Serialize, Deserialize)]
pub struct SetCommandArgs {
    pub barrier_model: String,
    pub barrier_algorithm: String,
}

#[derive(Serialize, Deserialize)]
pub struct Command {
    pub serial_number: String,
    pub command: String,
    pub authorization: String,
    pub arguments: Value,
}

#[derive(Serialize, Deserialize)]
pub struct SetData {
    pub serial_number: String,
}

#[derive(Serialize, Deserialize)]
pub struct Set {
    #[serde(rename = "type")]
    pub message_type: String,

    pub data: SetData,
}