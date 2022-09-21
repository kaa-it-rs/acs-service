use serde::{Deserialize, Serialize};

pub const HELLO_TYPE: &str = "HELLO";

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
