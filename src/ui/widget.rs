use crate::app::AppState;
use crate::common::{AsyncValue, SemanticKey};
use crate::domain::*;
use base64::Engine;
use eframe::egui;
use eframe::egui::{Context, TextureHandle, TextureOptions, Ui, WidgetText};
use std::cell::{OnceCell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

pub fn button_input(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    buffer: &mut String,
    on_click: impl Fn(&mut String),
) {
    ui.horizontal(|ui| {
        let input = ui.text_edit_singleline(buffer);
        if ui.button(text).clicked()
            || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            if !buffer.is_empty() {
                on_click(buffer);
            }
        }
    });
}

pub enum LabelInputKind {
    Text,
    Password,
}

pub fn label_input(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    buffer: &mut String,
    kind: LabelInputKind,
) {
    ui.label(text);
    match kind {
        LabelInputKind::Text => {
            ui.text_edit_singleline(buffer);
        }
        LabelInputKind::Password => {
            ui.add(egui::TextEdit::singleline(buffer).password(true));
        }
    }
}

pub fn captcha_button(
    ctx: &Context,
    ui: &mut Ui,
    app_state: &Rc<RefCell<dyn AppState>>,
    captcha_key: &OnceCell<SemanticKey>,
    captcha_id: &mut Option<CaptchaId>,
    captcha_texture: &mut Option<TextureHandle>,
    on_reload: impl Fn(),
) {
    if let Some(texture) = captcha_texture.as_ref() {
        let image_button = egui::ImageButton::new(texture);
        if ui.add(image_button).clicked() {
            *captcha_id = None;
            *captcha_texture = None;
            on_reload();
        }
    } else {
        match app_state
            .borrow()
            .get_captcha(captcha_key.get().unwrap().clone())
        {
            AsyncValue::Pending => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label("Loading captcha...");
                });
            }
            AsyncValue::Ready(Ok(data)) => {
                *captcha_id = Some(data.id);
                *captcha_texture = load_base64_texture(ctx, data.image.to_base64(), "captcha");
            }
            AsyncValue::Idle | AsyncValue::Ready(Err(_)) => {
                if ui.button("Reload captcha").clicked() {
                    on_reload();
                }
            }
        }
    }
}

fn load_base64_texture(ctx: &egui::Context, encoded: &str, name: &str) -> Option<TextureHandle> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let image_data = image::load_from_memory(&decoded).ok()?;
    let size = [image_data.width() as _, image_data.height() as _];
    let rgba = image_data.to_rgba8();
    let pixels = rgba.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    Some(ctx.load_texture(name, color_image, TextureOptions::default()))
}
