use std::cell::RefCell;
use std::rc::Rc;
use eframe::egui;
use tracing_subscriber::EnvFilter;
use client_side::infra::network::{Network, RealNetwork};
use client_side::state::RealAppState;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("debug,new_main=trace,client_side=trace"))
        .init();

    let network: Rc<RefCell<dyn Network>> = Rc::new(RefCell::new(RealNetwork::try_new().unwrap()));
    
    let (message_tx, message_rx) = crossbeam_channel::bounded(2048);
    let app_state = Rc::new(RefCell::new(RealAppState::new(message_tx.clone(), network)));
    let debug_state = None;

    let app = match client_side::app::EframeShell::try_new(app_state, debug_state, message_tx, message_rx) {
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
        "ClientSide",
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
