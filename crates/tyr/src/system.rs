pub use tyr_macros::system;

use crate::data::*;

pub type StartFn<D> = Box<dyn FnMut(*mut D) -> ()>;

pub struct System<D: Data> {
    name: String,
    access: D::Access,
    start: StartFn<D>,
}

impl<D: Data> System<D> {
    pub fn new(name: String, access: D::Access, start: StartFn<D>) -> Self {
        Self {
            name,
            access,
            start,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn access(&self) -> &D::Access {
        &self.access
    }

    pub unsafe fn start(&mut self, data: *mut D) {
        (self.start)(data)
    }
}
