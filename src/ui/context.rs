use eframe::egui;

pub trait ViewContext {}

impl ViewContext for egui::Context {}

pub fn downcast(ctx: &dyn ViewContext) -> &egui::Context {
    unsafe {
        let p = ctx as *const dyn ViewContext;
        let p = p as *const egui::Context;
        &*p
    }
}
