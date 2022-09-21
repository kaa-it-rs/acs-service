//! Describes messages from controllers

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HELLO_TYPE: &str = "HELLO";
pub const SET_TYPE: &str = "SET";
pub const ERROR_TYPE: &str = "ERROR";

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
    pub barrier_model: String,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Error {
    pub serial_number: String,
    pub code: u32,
    pub description: String,
    pub details: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Set {
    pub serial_number: String,
}

/// Error codes for controller's operations
#[allow(unused)]
enum ErrorCode {
    InternalServerError = 101,
    BadRequest = 102,
    Unauthorized = 103,
    Forbidden = 104,
    NotFound = 105,
    MethodNotAllowed = 106,
    NotImplemented = 107,
    ServiceUnavailable = 108,
    InvalidFirmwareFile = 109,
    PartiallySuccessful = 110,
    DataNotProvided = 111,
    NoSpaceForNewTags = 112,
    InvalidTag = 113,
}
