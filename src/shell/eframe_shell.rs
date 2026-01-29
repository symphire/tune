use crate::app::{AppMessage, AppState, DebugState};
use crate::ui;
use crate::ui::{DebugWindow, Window};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::Context;
use eframe::Frame;
use std::cell::RefCell;
use std::rc::Rc;

pub struct EframeShell {
    page_kind: ui::PageKind,
    page: Box<dyn ui::Page>,
    state: Rc<RefCell<dyn AppState>>,
    debug_window: Option<DebugWindow>,
    debug_state: Option<Rc<RefCell<dyn DebugState>>>,
    message_tx: Sender<AppMessage>,
    message_rx: Receiver<AppMessage>,
}

impl EframeShell {
    pub fn try_new(
        state: Rc<RefCell<dyn AppState>>,
        debug_state: Option<Rc<RefCell<dyn DebugState>>>,
        message_tx: Sender<AppMessage>,
        message_rx: Receiver<AppMessage>,
    ) -> anyhow::Result<EframeShell> {
        let debug_window = match &debug_state {
            None => None,
            Some(debug_state) => Some(DebugWindow::new(debug_state.clone())),
        };
        let page_kind = ui::PageKind::Login;
        let page = Self::make_page(page_kind, state.clone(), message_tx.clone());
        Ok(Self {
            page_kind,
            page,
            state,
            debug_window,
            debug_state,
            message_tx,
            message_rx,
        })
    }

    fn make_page(
        page_kind: ui::PageKind,
        app_state: Rc<RefCell<dyn AppState>>,
        message_tx: Sender<AppMessage>,
    ) -> Box<dyn ui::Page> {
        let mut page: Box<dyn ui::Page> = match page_kind {
            ui::PageKind::Lobby => Box::new(ui::LobbyPage::new(app_state, message_tx)),
            ui::PageKind::Login => Box::new(ui::LoginPage::new(app_state, message_tx)),
            ui::PageKind::Signup => Box::new(ui::SignupPage::new(app_state, message_tx)),
        };
        #[cfg(debug_assertions)]
        page.ensure_state();
        page.prepare_resource();
        page.init_message();
        page
    }

    pub fn switch_page(
        &mut self,
        page_kind: ui::PageKind,
        app_state: Rc<RefCell<dyn AppState>>,
        message_tx: Sender<AppMessage>,
    ) {
        self.page_kind = page_kind;
        self.page.drop_resource();
        self.page = Self::make_page(self.page_kind, app_state, message_tx);
    }
}

impl Drop for EframeShell {
    fn drop(&mut self) {
        self.page.drop_resource();
    }
}

impl eframe::App for EframeShell {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        for message in self.message_rx.try_iter() {
            let _ = self.state.borrow_mut().update(message);
        }

        if let Some(page) = self.page.view(ctx) {
            self.switch_page(page, self.state.clone(), self.message_tx.clone());
        }

        if let Some(debug_window) = &mut self.debug_window {
            debug_window.view(ctx);
        }

        ctx.request_repaint();
    }
}
