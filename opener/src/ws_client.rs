use anyhow::{bail, Result};
use futures_util::{future, pin_mut, StreamExt};
use thiserror::Error;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

mod dsl;

use dsl::{Hello, HelloData};
use crate::ws_client::dsl::{Set, SET_TYPE, SetData};

use self::dsl::HELLO_TYPE;

const PING_TIMEOUT: u64 = 60;
const PONG_WAIT_TIMEOUT: u64 = 5;
const RECONNECTION_TIMEOUT: u64 = 5;
const CONNECTION_TIMEOUT: u64 = 5;

#[derive(Error, Debug)]
#[error("Internal error")]
struct InternalError;

type SenderChannel = futures_channel::mpsc::UnboundedSender<Message>;
type PongReceiver = tokio::sync::mpsc::UnboundedReceiver<String>;
type PongSender = tokio::sync::mpsc::UnboundedSender<String>;
type ResetSender = tokio::sync::oneshot::Sender<i32>;
type StopPingSender = tokio::sync::oneshot::Sender<i32>;

/// Describes ws client for opener
pub struct WSClient {
    url: String,
    serial_number: String,
    model: String,
}

impl WSClient {
    pub fn new(address: &str, port: u16, serial_number: &str, model: &str) -> Self {
        let url = format!("ws://{address}:{port}/ws");
        WSClient {
            url,
            serial_number: serial_number.to_string(),
            model: model.to_string(),
        }
    }

    /// It is used for starting client
    pub async fn run(&self, stop: watch::Receiver<&'static str>) -> Result<()> {
        self.execute(stop).await
    }

    /// Handles PING and PONG Websocket messages
    async fn ping_sender(
        mut stop: watch::Receiver<&'static str>,
        message_sender: SenderChannel,
        mut pong_receiver: PongReceiver,
        reset_sender: ResetSender,
        mut stop_ping_sender: StopPingSender,
    ) -> Result<()> {
        loop {
            let before_ping_sleep = sleep(Duration::from_secs(PING_TIMEOUT));
            tokio::pin!(before_ping_sleep);

            log::info!("Wait before ping");

            tokio::select! {
                _ = &mut before_ping_sleep => {
                    log::info!("PING_TIMEOUT exceeded");
                }

                _ = stop_ping_sender.closed() => {
                    log::info!("Message output channel closed");
                    break;
                }

                 _ = stop.changed() => {
                    break;
                }
            }

            log::info!("Sending ping");

            message_sender.unbounded_send(Message::Ping("ping".as_bytes().to_vec()))?;

            log::info!("Ping sent");

            let pong_sleep = sleep(Duration::from_secs(PONG_WAIT_TIMEOUT));
            tokio::pin!(pong_sleep);

            tokio::select! {
                _ = &mut pong_sleep => {
                    if reset_sender.send(0).is_err() {
                        break;
                    }
                    log::info!("PONG_WAIT_TIMEOUT exceeded");
                    break;
                }

                _ = stop.changed() => {
                    break;
                }

                _ = pong_receiver.recv() => {
                    log::info!("Pong received");
                }
            }
        }

        log::info!("Ping sender task is finished");

        Ok(())
    }

    async fn execute(&self, mut stop: watch::Receiver<&'static str>) -> Result<()> {
        log::info!("Connect to: {}", self.url);

        let url = url::Url::parse(&self.url)?;

        'main: loop {
            //
            let (tx, rx) = futures_channel::mpsc::unbounded();

            let ws_stream;

            loop {
                let connection = connect_async(&url);

                let connection_timeout = sleep(Duration::from_secs(CONNECTION_TIMEOUT));
                tokio::pin!(connection_timeout);

                tokio::select! {
                    res = connection => {
                        match res {
                            Ok((s, _)) => {
                                ws_stream = s;
                                break;
                            },
                            Err(e) => {
                                log::error!("{}", e.to_string());
                            }
                        }
                    },

                    _ = &mut connection_timeout => {}
                }

                // To exit in case of Linux signal to interrupt

                let reconnection_sleep = sleep(Duration::from_secs(RECONNECTION_TIMEOUT));
                tokio::pin!(reconnection_sleep);

                tokio::select! {
                    _ = &mut reconnection_sleep => {},
                    _ = stop.changed() => return Ok(()),
                }
            }

            log::info!("WebSocket handshake has been successfully completed");

            let (write, read) = ws_stream.split();

            let (tx_pong, rx_pong) = tokio::sync::mpsc::unbounded_channel::<String>();

            let (tx_reset, rx_reset) = tokio::sync::oneshot::channel::<i32>();

            let to_ws = rx.map(Ok).forward(write);

            let tx_copy = tx.clone();
            let stop_copy = stop.clone();

            let (tx_stop_ping, mut rx_stop_ping) = tokio::sync::oneshot::channel::<i32>();

            let ping_sender_handle = tokio::spawn(async move {
                if let Err(e) =
                    Self::ping_sender(stop_copy, tx_copy, rx_pong, tx_reset, tx_stop_ping).await
                {
                    log::error!("Ping sender error: {}", e);
                }
            });

            if let Err(e) = self.send_hello(&tx).await {
                rx_stop_ping.close();
                ping_sender_handle.await?;
                return Err(e);
            }

            log::info!("Hello message sent");

            let from_ws = {
                read.for_each(|message| async {
                    if let Err(e) = self.process_package(message, &tx, &tx_pong).await {
                        let msg = e.to_string();
                        log::error!("{}", &msg);
                    }
                })
            };

            pin_mut!(from_ws, to_ws);
            let combined = future::select(from_ws, to_ws);

            tokio::select! {
                _ = combined => {
                    log::info!("Combined future selected");
                    rx_stop_ping.close();
                    ping_sender_handle.await?;
                }

                _ = rx_reset => {
                    log::info!("Reset future selected");
                    ping_sender_handle.await?;
                }

                _ = stop.changed() => {
                    ping_sender_handle.await?;
                    break 'main;
                }
            }

            log::info!("WebSocket client restarting...");
        }

        Ok(())
    }

    async fn process_package<E>(
        &self,
        message: std::result::Result<Message, E>,
        tx: &SenderChannel,
        tx_pong: &PongSender,
    ) -> Result<()>
    where
        E: std::error::Error + Sync + Send + 'static,
    {
        let message = message?;

        if message.is_pong() {
            tx_pong.send("pong".to_string())?;
            return Ok(());
        }

        let text = message.into_text()?;

        log::info!("Message received: ->{}<-", text);

        if text.is_empty() {
            return Ok(());
        }

        let command: dsl::Command = serde_json::from_str(&text)?;

        log::info!("Parsed test");

        match command.command.as_str() {
            "SET" => self.handle_set_command(&command, tx).await?,
            c => bail!("Unsupported command: {}", c),
        };

        Ok(())
    }

    async fn send_hello(&self, s: &SenderChannel) -> Result<()> {
        let hello = Hello {
            message_type: HELLO_TYPE.to_string(),
            data: HelloData {
                serial_number: self.serial_number.clone(),
                version: "1.0.2".to_string(),
                nonce: "jdfjksdhfjshfkjsdhkfhk".to_string(),
                barrier_model: self.model.clone(),
            },
        };

        let msg = serde_json::to_string(&hello).unwrap();

        log::info!("{}", msg.as_str());

        s.unbounded_send(Message::Text(msg)).unwrap();

        Ok(())
    }

    async fn handle_set_command(&self, req: &dsl::Command, s: &SenderChannel) -> Result<()> {
        let set = Set {
            message_type: SET_TYPE.to_string(),
            data: SetData {
                serial_number: self.serial_number.clone(),
            },
        };

        let msg = serde_json::to_string(&set).unwrap();

        log::info!("{}", msg.as_str());

        s.unbounded_send(Message::Text(msg)).unwrap();

        Ok(())
    }
}
