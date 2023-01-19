pub use tyr_macros::system;

use futures::future::BoxFuture;
use crate::data::*;

pub type StartFn<D> = Box<dyn FnMut(*mut D) -> BoxFuture<'static, ()>>;

pub struct System<D: Data> {
    name: String,
    access: D::Access,
    run: StartFn<D>,
}

impl<D: Data> System<D> {
    pub fn new(name: String, access: D::Access, run: StartFn<D>) -> Self {
        Self {
            name,
            access,
            run,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn access(&self) -> &D::Access {
        &self.access
    }

    pub unsafe fn run(&mut self, data: *mut D) -> BoxFuture<()> {
        (self.run)(data)
    }
}
