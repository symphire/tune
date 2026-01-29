use crate::ui::ViewContext;

#[derive(Debug, Copy, Clone)]
pub enum PageKind {
    Lobby,
    Login,
    Signup,
}

pub trait Page {
    /// Ensures shared state match this page’s invariant.
    #[cfg(debug_assertions)]
    fn ensure_state(&self) {}
    fn prepare_resource(&mut self) {}
    fn drop_resource(&mut self) {}
    fn init_message(&self) {}
    fn view(&mut self, ctx: &dyn ViewContext) -> Option<PageKind>;
}
