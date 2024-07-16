use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    system::{NormalSystem, System},
    Inspect,
};

use miette::{miette, Result};

pub type BoxedSystem = Box<dyn System<NormalSystem> + 'static>;

/// A thread-safe container that holds one instance of type `T`
#[derive(Debug, Default)]
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

impl<T: Send + Sync + 'static> Clone for Resource<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
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

/// A type-erased resource that is inspectable.
#[derive(Clone)]
pub struct InspectableResource(Arc<RwLock<dyn Inspect + Send + Sync + 'static>>);

impl<T: Inspect + Send + Sync + 'static> From<Resource<T>> for InspectableResource {
    fn from(resource: Resource<T>) -> Self {
        Self(resource.value)
    }
}

impl InspectableResource {
    pub fn read(&self) -> LockResult<RwLockReadGuard<dyn Inspect + Send + Sync + 'static>> {
        self.0.read()
    }

    pub fn write(&self) -> LockResult<RwLockWriteGuard<dyn Inspect + Send + Sync + 'static>> {
        self.0.write()
    }
}

/// Wrapper around a [`HashMap`](`std::collections::HashMap`) that stores type-erased pointers to a resource, based on their [`TypeId`](`std::any::TypeId`).
#[derive(Debug, Default, Clone)]
pub struct Storage(HashMap<TypeId, ErasedResource>);

impl Storage {
    /// Create a new resource storage.
    pub fn new() -> Self {
        let map = HashMap::from([(
            TypeId::of::<InspectView>(),
            Resource::new(InspectView::new()).into(),
        )]);

        Storage(map)
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
                "Trying to add resource of type `{}`, but it already exists in storage! Only 1 resource can exist per type.",
                std::any::type_name::<T>()
            )),
            None => Ok(()),
        }
    }

    /// Consumes the [`Resource<T>`] and adds it to the storage as well as the debug pointer list.
    ///
    /// Internally, this works by allocating it on the heap and storing a type-erased pointer
    /// in a [`HashMap`](`std::collections::HashMap`) based on the type's [`TypeId`](`std::any::TypeId`).
    ///
    /// # Errors
    /// This function fails if there is already a resource of type `T` in the storage.
    pub fn add_inspectable_resource<T: Inspect + Send + Sync + 'static>(
        &mut self,
        res: Resource<T>,
    ) -> Result<()> {
        match self.0.insert(TypeId::of::<T>(), res.clone().into()) {
            Some(_) => Err(miette!(
                "Trying to add resource of type `{}`, but it already exists in storage! Only 1 resource can exist per type.",
                std::any::type_name::<T>()
            )),
            None => {
                self.map_resource_mut(|view: &mut InspectView| view.push(res.into()))
                    .unwrap();
                Ok(())
            }
        }
    }

    /// Try to get a resource based on `T` its [`std::any::TypeId`].
    ///   
    /// Returns `None` if the type does not exist in the storage.
    pub(super) fn get<T: 'static>(&self) -> Option<&ErasedResource> {
        let type_id = TypeId::of::<T>();
        self.0.get(&type_id)
    }

    /// Try to get a resource from the storage by reference, and map it to something else
    #[allow(dead_code)]
    pub(super) fn map_resource_ref<T: 'static, F: FnOnce(&T) -> R, R>(&self, f: F) -> Result<R> {
        let resource = self
            .get::<T>()
            .ok_or_else(|| miette!("Resource of type `{}` does not exist", type_name::<T>()))?;

        let guard = resource
            .read()
            .unwrap_or_else(|_| panic!("Failed to lock resource of type `{}`", type_name::<&T>()));

        Ok(f(guard.downcast_ref().unwrap()))
    }

    /// Try to get a resource from the storage by mutable reference, and map it to something else
    #[allow(dead_code)]
    pub(super) fn map_resource_mut<T: 'static, F: FnOnce(&mut T) -> R, R>(
        &self,
        f: F,
    ) -> Result<R> {
        let resource = self
            .get::<T>()
            .ok_or_else(|| miette!("Resource of type `{}` does not exist", type_name::<T>()))?;

        let mut guard = resource.write().unwrap_or_else(|_| {
            panic!(
                "Failed to lock resource of type `{}`",
                type_name::<&mut T>()
            )
        });

        Ok(f(guard.downcast_mut().unwrap()))
    }
}

#[derive(Default)]
pub struct InspectView(Vec<InspectableResource>);

impl InspectView {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn resources(&self) -> impl Iterator<Item = &InspectableResource> {
        self.0.iter()
    }

    pub fn by_index(&self, index: usize) -> Option<&InspectableResource> {
        self.0.get(index)
    }

    pub fn by_name(&self, name: &str) -> Option<&InspectableResource> {
        self.0.iter().find(|r| r.read().unwrap().name() == name)
    }

    fn push(&mut self, res: InspectableResource) {
        self.0.push(res)
    }
}
