use miette::Result;
use tyr::prelude::*;

use super::runtime::AsyncTask;

pub trait AsyncResource {
    fn add_async_resource<T: Send + Sync + 'static>(self, resource: Resource<T>) -> Result<Self>
    where
        Self: Sized;
}

impl AsyncResource for App {
    fn add_async_resource<T: Send + Sync + 'static>(self, resource: Resource<T>) -> Result<Self>
    where
        Self: Sized,
    {
        self.add_resource(Resource::new(AsyncTask::<T>::default()))?
            .add_resource(resource)
    }
}
