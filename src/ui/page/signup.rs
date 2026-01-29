use crate::app::{AppMessage, AppState};
use crate::common::{AsyncValue, SemanticKey};
use crate::domain::*;
use crate::ui::widget::LabelInputKind;
use crate::ui::*;
use crossbeam_channel::Sender;
use eframe::egui;
use eframe::egui::{TextBuffer, TextureHandle};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

pub struct SignupPage {
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

impl SignupPage {
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

impl Page for SignupPage {
    fn prepare_resource(&mut self) {
        self.captcha_key
            .set(self.app_state.borrow_mut().prepare_captcha())
            .expect("captcha_key already set");
        self.app_state.borrow_mut().prepare_signup_state();
    }

    fn drop_resource(&mut self) {
        let key = self.captcha_key.take().expect("captcha_key not set");
        self.app_state.borrow_mut().drop_captcha(key);
        self.app_state.borrow_mut().drop_signup_state();
    }

    fn init_message(&self) {
        let _ = self.message_tx.send(AppMessage::CaptchaRequest(
            self.captcha_key.get().unwrap().clone(),
        ));
    }

    fn view(&mut self, ctx: &dyn ViewContext) -> Option<PageKind> {
        let ctx = downcast(ctx);

        let mut next_page = None;

        egui::Window::new("Signup")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                widget::label_input(
                    ui,
                    "Username:",
                    &mut self.username_buf,
                    LabelInputKind::Text,
                );
                widget::label_input(
                    ui,
                    "Password:",
                    &mut self.password_buf,
                    LabelInputKind::Password,
                );
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
                    if ui.button("Go to login").clicked() {
                        next_page = Some(PageKind::Login);
                    }

                    let app_state = self.app_state.borrow();
                    let signup_state = app_state.get_signup_state();
                    let can_submit = !matches!(signup_state, AsyncValue::Pending);
                    if ui
                        .add_enabled(can_submit, egui::Button::new("Submit"))
                        .clicked()
                    {
                        let _ = self.message_tx.send(AppMessage::SignupRequest(SignupInput {
                            username: self.username_buf.take(),
                            password: self.password_buf.take(),
                            captcha_id: self.captcha_id.expect("captcha_id not set"),
                            captcha_answer: self.captcha_buf.take(),
                        }));
                    }
                    match signup_state {
                        AsyncValue::Idle => {}
                        AsyncValue::Pending => {
                            ui.spinner();
                            ui.label("Creating user profile...");
                        }
                        AsyncValue::Ready(Ok(_)) => {
                            ui.label("New user created. Please go to login.");
                        }
                        AsyncValue::Ready(Err(_)) => {
                            ui.label("Signup failed. Please retry.");
                        }
                    }
                })
            });

        next_page
    }
}
