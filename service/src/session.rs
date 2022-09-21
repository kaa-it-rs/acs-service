use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web_actors::ws;

mod dsl;

use dsl::{HELLO_TYPE, SET_TYPE, ERROR_TYPE};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct WsOpenerSession {
    pub id: Option<String>,
    pub hb: Instant,
    pub addr: Addr<super::server::OpenerServer>,
}

impl Actor for WsOpenerSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("Session started");
        self.hb(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        if let Some(x) = &self.id {
            self.addr
                .do_send(super::server::message::Disconnect { id: x.clone() });
        }

        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsOpenerSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        log::info!("{:?}", msg);
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }

            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }

            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }

            ws::Message::Text(text) => {
                log::info!("Raw msg: {}", text);

                let msg: dsl::Response = match serde_json::from_str(&text) {
                    Err(e) => {
                        log::error!("Parse message failed: {}", e);
                        return;
                    }
                    Ok(msg) => msg,
                };

                match msg.response_type.as_str() {
                    HELLO_TYPE => {
                        let hello: dsl::Hello = match serde_json::from_value(msg.data) {
                            Err(e) => {
                                log::error!("Wrong hello message format: {}", e);
                                return;
                            }
                            Ok(hello) => hello,
                        };

                        log::info!("Hello message: {}", serde_json::to_string(&hello).unwrap());

                        let addr = ctx.address();

                        self.addr
                            .send(super::server::message::Connect {
                                addr: addr.recipient(),
                                serial_number: hello.serial_number,
                                version: hello.version,
                                nonce: hello.nonce,
                                barrier_model: hello.barrier_model,
                            })
                            .into_actor(self)
                            .then(|res, act, ctx| {
                                match res {
                                    Ok(res) => match res {
                                        Ok(id) => act.id = Some(id),
                                        Err(_) => ctx.stop(),
                                    },
                                    Err(e) => {
                                        log::error!("Error on server connect: {}", e);
                                        ctx.stop()
                                    }
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    }

                    // SET_TYPE => {
                    //   let set: dsl::Set = match serde_json::from_value(msg.data) {
                    //     Err(e) => {
                    //       log::error!("Wrong set message format: {}", e);
                    //       return;
                    //     }
                    //     Ok(set) => set,
                    //   };
                    //
                    //   log::info!("Set message: {}", serde_json::to_string(&set).unwrap());
                    //
                    //   let addr = ctx.address();
                    //
                    //   self.addr
                    //     .send(super::server::message::Set {
                    //       serial_number: set.serial_number,
                    //     })
                    //     .into_actor(self)
                    //     .then(|res, act, ctx| {
                    //       match res {
                    //         Ok(res) => match res {
                    //           Err(e) => log::error!("Error on server set handler: {}", e),
                    //           _ => (),
                    //         }
                    //         Err(e) => log::info!("Failed to send set message to server: {}", e),
                    //       }
                    //       fut::ready(())
                    //     })
                    //     .wait(ctx);
                    // }
                    //
                    // ERROR_TYPE => {
                    //   let error: dsl::Error = match serde_json::from_value(msg.data) {
                    //     Err(e) => {
                    //       log::error!("Wrong error message format: {}", e);
                    //       return;
                    //     }
                    //     Ok(error) => error,
                    //   };
                    //
                    //   log::info!("Error message: {}", serde_json::to_string(&error).unwrap());
                    //
                    //   let addr = ctx.address();
                    //
                    //   self.addr
                    //     .send(super::server::message::Error {
                    //       serial_number: error.serial_number,
                    //       code: error.code,
                    //       description: error.description,
                    //       details: error.details,
                    //     })
                    //     .into_actor(self)
                    //     .then(|res, act, ctx| {
                    //       match res {
                    //         Ok(res) => match res {
                    //           Err(e) => log::error!("Error on server error handler: {}", e),
                    //           _ => (),
                    //         }
                    //         Err(e) => log::info!("Failed to send error message to server: {}", e),
                    //       }
                    //       fut::ready(())
                    //     })
                    //     .wait(ctx);
                    // }

                    t => {
                        log::error!("Unsupported message type: {}", t);
                    }
                }
            }

            ws::Message::Binary(_) => {}

            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }

            ws::Message::Continuation(_) => {
                ctx.stop();
            }

            ws::Message::Nop => (),
        }
    }
}

impl WsOpenerSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("WebSocket Client heartbeat failed, disconnecting!");

                if let Some(id) = &act.id {
                    act.addr
                        .do_send(super::server::message::Disconnect { id: id.clone() });
                }

                ctx.stop();

                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Handler<super::server::command::SetCommand> for WsOpenerSession {
    type Result = ();

    fn handle(&mut self, cmd: super::server::command::SetCommand, ctx: &mut Self::Context) {
        let msg = match serde_json::to_string(&cmd) {
            Err(e) => {
                log::error!("Failed to serialize set command: {}", e);
                return;
            }
            Ok(msg) => msg
        };

        ctx.text(msg);
    }
}
