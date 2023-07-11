use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::system::System;

use color_eyre::{eyre::eyre, Result};

pub type BoxedSystem = Box<dyn System + 'static>;

#[derive(Debug, Default, Clone)]
pub struct Resource<T: Send + Sync + 'static> {
    value: Arc<RwLock<T>>,
}

impl<T: Send + Sync + 'static> Resource<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
        }
    }
}

impl<T: Send + Sync + 'static> From<Resource<T>> for ErasedResource {
    fn from(resource: Resource<T>) -> Self {
        Self(resource.value)
    }
}

/// A type-erased resource that can be accessed in parallel by systems
#[derive(Debug, Clone)]
pub struct ErasedResource(Arc<RwLock<dyn Any + Send + Sync + 'static>>);

impl ErasedResource {
    pub fn read(&self) -> LockResult<RwLockReadGuard<dyn Any + Send + Sync + 'static>> {
        self.0.read()
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<dyn Any + Send + Sync + 'static>> {
        self.0.write()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Storage(HashMap<TypeId, ErasedResource>);

impl Storage {
    pub fn new() -> Self {
        Storage(HashMap::new())
    }

    /// Consumes the [`Resource<T>`] and adds it to the storage by turning it into a storable [`ErasedResource`]
    pub fn add_resource<T: Send + Sync + 'static>(&mut self, res: Resource<T>) -> Result<()> {
        match self.0.insert(TypeId::of::<T>(), res.into()) {
            Some(_) => Err(eyre!(
                "Trying to add resource of type `{}`, but it already exists in storage!",
                std::any::type_name::<T>()
            )),
            None => Ok(()),
        }
    }

    pub fn get(&self, key: &TypeId) -> Option<&ErasedResource> {
        self.0.get(key)
    }
}
