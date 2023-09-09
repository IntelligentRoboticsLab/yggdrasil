use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::system::System;

use miette::{miette, Result};

pub type BoxedSystem = Box<dyn System + 'static>;

/// A thread-safe container that holds one instance of type `T`
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

/// Wrapper around a [`HashMap`](`std::collections::HashMap`) that stores type-erased pointers to a resource, based on their [`TypeId`](`std::any::TypeId`).
#[derive(Debug, Default, Clone)]
pub struct Storage(HashMap<TypeId, ErasedResource>);

impl Storage {
    /// Create a new resource storage.
    pub fn new() -> Self {
        Storage(HashMap::new())
    }

    /// Consumes the [`Resource<T>`] and adds it to the storage.
    ///
    /// Internally, this works by allocating it on the heap and storing a type-erased pointer
    /// in a [`HashMap`](`std::collections::HashMap`) based on the type's [`TypeId`](`std::any::TypeId`).
    ///
    /// # Errors
    /// This function fails if there is already a resource of type `T` in the storage.
    pub fn add_resource<T: Send + Sync + 'static>(&mut self, res: Resource<T>) -> Result<()> {
        match self.0.insert(TypeId::of::<T>(), res.into()) {
            Some(_) => Err(miette!(
                "Trying to add resource of type `{}`, but it already exists in storage!",
                std::any::type_name::<T>()
            )),
            None => Ok(()),
        }
    }

    /// Try to get a resource based on `T` its [`std::any::TypeId`].
    ///
    /// Returns `None` if the type does not exist in the storage.
    pub fn get<T: 'static>(&self) -> Option<&ErasedResource> {
        let type_id = TypeId::of::<T>();
        self.0.get(&type_id)
    }

    /// TODO: docs
    pub fn map_resource_ref<T: 'static, F: Fn(&T) -> R, R>(&self, f: F) -> R {
        let resource = self
            .get::<T>()
            .unwrap_or_else(|| panic!("Resource of type `{}` does not exist", type_name::<T>()));
        let guard = resource
            .read()
            .unwrap_or_else(|_| panic!("Failed to lock resource of type `{}`", type_name::<T>()));

        f(guard.downcast_ref().unwrap())
    }

    /// TODO: docs
    pub fn map_resource_mut<T: 'static, F: Fn(&mut T) -> R, R>(&self, f: F) -> R {
        let resource = self
            .get::<T>()
            .unwrap_or_else(|| panic!("Resource of type `{}` does not exist", type_name::<T>()));
        let mut guard = resource
            .write()
            .unwrap_or_else(|_| panic!("Failed to lock resource of type `{}`", type_name::<T>()));

        f(guard.downcast_mut().unwrap())
    }
}
