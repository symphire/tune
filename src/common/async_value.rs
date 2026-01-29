#[derive(Debug, Clone)]
pub enum AsyncValue<T, E> {
    Idle,
    Pending,
    Ready(Result<T, E>),
}
