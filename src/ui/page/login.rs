use crate::domain::*;
use crate::ui::*;
use crossbeam_channel::Sender;
use eframe::egui;
use eframe::egui::{TextBuffer, TextureHandle};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;
use crate::common::{AsyncValue, SemanticKey};
use crate::ui::widget::LabelInputKind;

pub struct LoginPage {
    // global
    app_state: Rc<RefCell<dyn AppState>>,
    message_tx: Sender<AppMessage>,
    // input
    username_buf: String,
    password_buf: String,
    captcha_buf: String,
    // display
    captcha_key: OnceCell<SemanticKey>,
    captcha_id: Option<CaptchaId>,
    captcha_texture: Option<TextureHandle>,
}

impl LoginPage {
    pub fn new(app_state: Rc<RefCell<dyn AppState>>, message_tx: Sender<AppMessage>) -> Self {
        Self {
            app_state,
            message_tx,
            username_buf: String::new(),
            password_buf: String::new(),
            captcha_buf: String::new(),
            captcha_key: OnceCell::new(),
            captcha_id: None,
            captcha_texture: None,
        }
    }
}

impl Page for LoginPage {
    fn prepare_resource(&mut self) {
        self.captcha_key
            .set(self.app_state.borrow_mut().prepare_captcha())
            .expect("captcha_key already set");
        self.app_state.borrow_mut().prepare_login_state();
    }

    fn drop_resource(&mut self) {
        let key = self.captcha_key.take().expect("captcha_key not set");
        self.app_state
            .borrow_mut()
            .drop_captcha(key);
        self.app_state.borrow_mut().drop_login_state();
    }

    fn init_message(&self) {
        let _ = self.message_tx.send(AppMessage::CaptchaRequest(
            self.captcha_key.get().unwrap().clone(),
        ));
    }

    fn view(&mut self, ctx: &dyn ViewContext) -> Option<PageKind> {
        let ctx = downcast(ctx);

        let mut next_page = None;

        egui::Window::new("Login")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                widget::label_input(ui, "Username:", &mut self.username_buf, LabelInputKind::Text);
                widget::label_input(ui, "Password:", &mut self.password_buf, LabelInputKind::Password);
                widget::label_input(ui, "Captcha:", &mut self.captcha_buf, LabelInputKind::Text);

                widget::captcha_button(
                    ctx,
                    ui,
                    &self.app_state,
                    &self.captcha_key,
                    &mut self.captcha_id,
                    &mut self.captcha_texture,
                    || {
                        let key = self.captcha_key.get().unwrap().clone();
                        let _ = self.message_tx.send(AppMessage::CaptchaRequest(key));
                    },
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Go to signup").clicked() {
                        next_page = Some(PageKind::Signup);
                    }
                    
                    match self.app_state.borrow().get_login_state() {
                        AsyncValue::Idle => {
                            if ui.button("Submit").clicked() {
                                let _ = self.message_tx.send(AppMessage::LoginRequest(LoginInput {
                                    username: self.username_buf.take(),
                                    password: self.password_buf.take(),
                                    captcha_id: self.captcha_id.expect("captcha_id not set (login)"),
                                    captcha_answer: self.captcha_buf.take(),
                                }));
                            }
                        }
                        AsyncValue::Pending => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Waiting for authentication...");
                            });
                        }
                        AsyncValue::Ready(result) => match result {
                            Ok(_) => next_page = Some(PageKind::Lobby),
                            Err(_) => {
                                if ui.button("Submit").clicked() {
                                    let _ = self.message_tx.send(AppMessage::LoginRequest(LoginInput {
                                        username: self.username_buf.take(),
                                        password: self.password_buf.take(),
                                        captcha_id: self.captcha_id.expect("captcha_id not set (login)"),
                                        captcha_answer: self.captcha_buf.take(),
                                    }));
                                }
                                ui.label("Login failed. Please retry.");
                            }
                        },
                    }
                });
            });

        next_page
    }
}
