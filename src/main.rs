use std::cell::RefCell;
use std::rc::Rc;
use clap::{Parser};
use eframe::egui;
use tracing_subscriber::EnvFilter;
use tune::*;
use tune::infra::network::{Network, RealNetwork};
use tune::state::RealAppState;

fn main() {
    let args = shell::Args::parse();

    let log_config = format!("debug,tune={}", args.log_level);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_config))
        .init();

    let network: Rc<RefCell<dyn Network>> = Rc::new(RefCell::new(RealNetwork::try_new().unwrap()));

    let (message_tx, message_rx) = crossbeam_channel::bounded(2048);
    let app_state = Rc::new(RefCell::new(RealAppState::new(message_tx.clone(), network)));
    let debug_state = None;

    let app = match tune::app::EframeShell::try_new(app_state, debug_state, message_tx, message_rx) {
        Ok(app) => {
            tracing::debug!("AppShell created successfully");
            app
        },
        Err(e) => {
            tracing::error!("failed to create AppShell: {e}");
            return;
        }
    };

    if let Err(e) = eframe::run_native(
        "Tune",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 720.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(app))),
    ) {
        tracing::error!("{e}");
    }
}