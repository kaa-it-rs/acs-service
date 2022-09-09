use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HELLO_TYPE: &str = "HELLO";

#[derive(Deserialize)]
pub(super) struct Response {
    #[serde(rename = "type")]
    pub response_type: String,

    pub data: Value,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Hello {
    pub serial_number: String,
    pub version: String,
    pub nonce: String,
}
