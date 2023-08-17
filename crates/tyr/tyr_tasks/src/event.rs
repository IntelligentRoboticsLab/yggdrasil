pub trait Event<T> {
    type Data;

    fn spawn(&mut self, data: Self::Data);
    fn is_alive(&self) -> bool;
    fn poll(&mut self) -> Option<T>;
    fn kill(&mut self);
}
