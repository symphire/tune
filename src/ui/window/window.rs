use crate::ui::ViewContext;

#[derive(Debug, Copy, Clone)]
pub enum WindowKind {
    Debug,
}

pub trait Window {
    fn view(&mut self, ctx: &dyn ViewContext);
}
