use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message, Clone)]
#[rtype(result = "Result<String, String>")]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub serial_number: String,
    pub version: String,
    pub nonce: String,
}

#[derive(Message, Clone)]
#[rtype(result = "Result<(), String>")]
pub struct Disconnect {
    pub id: String,
}
