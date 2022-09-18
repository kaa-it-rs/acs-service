use futures_util::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook_tokio::{Handle, Signals};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

lazy_static! {
    static ref APP: Arc<App> = Arc::new(App::new());
}

/// Describes application instance errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Signals already registered")]
    SignalsAlreadyRegistered,

    #[error("Signals already unregistered")]
    SignalsAlreadyUnregistered,
}

// Describes info about Linux signals subscription
struct SignalsInfo {
    handle: Handle,
    signals_task: JoinHandle<()>,
}

/// Describes single instance of application
pub struct App {
    signals_info: Mutex<Option<SignalsInfo>>,
}

impl App {
    pub fn instance() -> Arc<App> {
        APP.clone()
    }

    fn new() -> Self {
        App {
            signals_info: Mutex::new(None),
        }
    }

    // Handler for Linux signals
    async fn handle_signals(signals: Signals, stop: watch::Sender<&'static str>) {
        let mut signals = signals.fuse();
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM | SIGINT => {
                    let _ = stop.send("close");
                }
                _ => unreachable!(),
            }
        }
    }

    /// Allows to register for SIGINT and SEGTERM Linux signals
    pub async fn signals_register(&self) -> Result<watch::Receiver<&'static str>, AppError> {
        let mut signals_info = self.signals_info.lock().await;

        if signals_info.is_some() {
            return Err(AppError::SignalsAlreadyRegistered);
        }

        let signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
        let handle = signals.handle();

        let (twx, rwx) = watch::channel("work");

        let signals_task = tokio::spawn(App::handle_signals(signals, twx));

        *signals_info = Some(SignalsInfo {
            handle,
            signals_task,
        });

        Ok(rwx)
    }

    /// Allows to unregister from registered Linux signals
    pub async fn signals_unregister(&self) -> Result<(), AppError> {
        let mut signals_info = self.signals_info.lock().await;

        if signals_info.is_none() {
            return Err(AppError::SignalsAlreadyUnregistered);
        }

        let si = signals_info.take().unwrap();

        si.handle.close();
        si.signals_task.await.unwrap();

        Ok(())
    }
}
