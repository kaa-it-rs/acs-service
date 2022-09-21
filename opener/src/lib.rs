#[macro_use]
extern crate lazy_static;

use anyhow::Result;

mod app;
mod ws_client;

/// Runs the application
pub async fn run(address: &str, port: u16, serial_number: &str, model: &str) {
    let res = run_internal(address, port, serial_number, model).await;
    if let Err(e) = res {
        log::error!("{}", e.to_string());
    }
}

async fn run_internal(address: &str, port: u16, serial_number: &str, model: &str) -> Result<()> {
    log::info!("Opener started");

    let a = app::App::instance();

    let signals_rx = a.signals_register().await?;

    log::info!("Registered to Linux signals");

    log::info!("Starting ws client");

    let client = ws_client::WSClient::new(address, port, serial_number, model);
    client.run(signals_rx).await?;

    log::info!("WS client is stopped");

    a.signals_unregister().await?;

    log::info!("Unregistered from Linux signals");

    Ok(())
}
