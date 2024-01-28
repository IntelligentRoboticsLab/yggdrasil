use dyn_clone::DynClone;
use miette::Result;
use std::hash::Hash;
use std::sync::RwLock;
use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::storage::{ErasedResource, Storage};

pub trait System<const STARTUP: bool>: DynClone + Send + Sync + 'static {
    fn run(&mut self, resources: &mut Storage) -> Result<()>;
    fn required_resources(&self) -> Vec<TypeInfo>;
    fn system_type(&self) -> TypeId;
    fn system_name(&self) -> &str;
}

dyn_clone::clone_trait_object!(<const STARTUP: bool> System<STARTUP>);

macro_rules! impl_system {
    (
        $($params:ident),*
    ) => {
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> System<false> for FunctionSystem<($($params,)*), F>
            where
                for<'a, 'b> &'a mut F:
                    FnMut( $($params),* ) -> Result<()> +
                    FnMut( $(<$params as SystemParam>::Item<'b>),* ) -> Result<()>
        {
            fn run(&mut self, resources: &mut Storage) -> Result<()> {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<$($params),*> (
                    mut f: impl FnMut($($params),*) -> Result<()>,
                    $($params: $params),*
                ) -> Result<()> {
                    f($($params),*)
                }

                $(
                    let $params = $params::get_resource(&resources);
                )*

                {
                    $(

                        let $params = $params::retrieve(&$params);
                    )*

                    call_inner(&mut self.f, $($params),*)
                }
            }

            fn required_resources(&self) -> Vec<TypeInfo> {
                let mut types = Vec::new();

                $(
                    let mut param_types: Vec<TypeInfo> = <$params as SystemParam>::type_info()
                        .into_iter()
                        .collect();

                    types.append(&mut param_types);
                )*

                types
            }

            fn system_type(&self) -> TypeId {
                TypeId::of::<($($params,)*)>()
            }

            fn system_name(&self) -> &str {
                std::any::type_name::<F>()
            }

        }

        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> System<true> for FunctionSystem<($($params,)*), F>
            where
                for<'a, 'b> &'a mut F:
                FnOnce( &mut Storage, $($params),* ) -> Result<()> +
                FnOnce( &mut Storage, $(<$params as SystemParam>::Item<'b>),* ) -> Result<()>
        {
            fn run(&mut self, resources: &mut Storage) -> Result<()> {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<$($params),*> (
                    mut f: impl FnOnce(&mut Storage, $($params),*) -> Result<()>,
                    storage: &mut Storage,
                    $($params: $params),*
                ) -> Result<()> {
                    f(storage, $($params),*)
                }
                    $(
                        let $params = $params::get_resource(&resources);
                    )*

                    {
                        $(
                            let $params = $params::retrieve(&$params);
                        )*


                        call_inner(&mut self.f, resources, $($params),*)
                    }
            }

            fn required_resources(&self) -> Vec<TypeInfo> {
                let mut types = Vec::new();

                $(
                    let mut param_types: Vec<TypeInfo> = <$params as SystemParam>::type_info()
                        .into_iter()
                        .collect();

                    types.append(&mut param_types);
                )*

                types
            }

            fn system_type(&self) -> TypeId {
                TypeId::of::<($($params,)*)>()
            }

            fn system_name(&self) -> &str {
                std::any::type_name::<F>()
            }

        }
    }
}

impl_system!();
impl_system!(T1);
impl_system!(T1, T2);
impl_system!(T1, T2, T3);
impl_system!(T1, T2, T3, T4);
impl_system!(T1, T2, T3, T4, T5);
impl_system!(T1, T2, T3, T4, T5, T6);
impl_system!(T1, T2, T3, T4, T5, T6, T7);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8);

pub struct FunctionSystemTypes<Input: 'static>(PhantomData<fn() -> Input>);

impl<T> Clone for FunctionSystemTypes<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T> PartialEq for FunctionSystemTypes<T> {
    fn eq(&self, _other: &Self) -> bool {
        // Always true when `T` is the same
        true
    }
}

impl<T> Eq for FunctionSystemTypes<T> {}

pub struct FunctionSystem<Input: 'static, F: Clone + 'static> {
    f: F,
    // The set of types requested by the system
    // we need a marker because otherwise we're not using `Input`.
    // fn() -> Input is chosen because just using Input would not be `Send` + `Sync`,
    // but the fnptr is always `Send` + `Sync`.
    _marker: FunctionSystemTypes<Input>,
}

impl<Input: 'static, F: Clone + 'static> Clone for FunctionSystem<Input, F> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _marker: self._marker.clone(),
        }
    }
}

pub trait IntoSystem<const STARTUP: bool, Input>: Clone {
    type System: System<STARTUP>;

    fn into_system(self) -> Self::System;
}

macro_rules! impl_into_system {
    (
        $($params:ident),*
    ) => {
        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> IntoSystem<false, ($($params,)*)> for F
            where
                for<'a, 'b> &'a mut F:
                    FnMut( $($params),* ) -> Result<()> +
                    FnMut( $(<$params as SystemParam>::Item<'b>),* )  -> Result<()>
        {
            type System = FunctionSystem<($($params,)*), Self>;

            fn into_system(self) -> Self::System {
                FunctionSystem {
                    f: self,
                    _marker: FunctionSystemTypes(PhantomData),
                }
            }
        }

        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> IntoSystem<true, ($($params,)*)> for F
            where
                for<'a, 'b> &'a mut F:
                    FnOnce( &mut Storage, $($params),* ) -> Result<()> +
                    FnOnce( &mut Storage, $(<$params as SystemParam>::Item<'b>),* )  -> Result<()>
        {
            type System = FunctionSystem<($($params,)*), Self>;

            fn into_system(self) -> Self::System {
                FunctionSystem {
                    f: self,
                    _marker: FunctionSystemTypes(PhantomData),
                }
            }
        }
    }
}

impl_into_system!();
impl_into_system!(T1);
impl_into_system!(T1, T2);
impl_into_system!(T1, T2, T3);
impl_into_system!(T1, T2, T3, T4);
impl_into_system!(T1, T2, T3, T4, T5);
impl_into_system!(T1, T2, T3, T4, T5, T6);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8);

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TypeInfo {
    pub id: TypeId,
    pub name: String,
}

impl TypeInfo {
    fn new(id: TypeId, name: String) -> Self {
        Self { id, name }
    }
}

pub trait SystemParam {
    type Item<'new>;

    fn retrieve(resource: &RwLock<dyn Any + Send + Sync>) -> Self::Item<'_>;
    fn get_resource(storage: &Storage) -> ErasedResource;
    fn type_info() -> Vec<TypeInfo>;
}

/// Immutable access to a [`Resource<T>`](`crate::Resource<T>`).
pub struct Res<'a, T: Send + Sync + 'static> {
    value: RwLockReadGuard<'a, dyn Any + Send + Sync>,
    _marker: PhantomData<T>,
}

impl<'a, T: Send + Sync + 'static> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.downcast_ref::<T>().expect("L")
    }
}

impl<'res, T: Send + Sync + 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;

    fn retrieve(resource: &RwLock<dyn Any + Send + Sync>) -> Self::Item<'_> {
        Res {
            value: resource.read().unwrap(),
            _marker: PhantomData,
        }
    }

    fn get_resource(storage: &Storage) -> ErasedResource {
        storage.get::<T>().unwrap().clone()
    }

    fn type_info() -> Vec<TypeInfo> {
        vec![TypeInfo::new(
            TypeId::of::<T>(),
            type_name::<T>().to_owned(),
        )]
    }
}

/// Mutable access to a [`Resource<T>`](`crate::Resource<T>`).
pub struct ResMut<'a, T: Send + Sync + 'static> {
    value: RwLockWriteGuard<'a, dyn Any + Send + Sync>,
    _marker: PhantomData<T>,
}

impl<'a, T: Send + Sync + 'static> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.downcast_ref::<T>().unwrap()
    }
}

impl<'a, T: Send + Sync + 'static> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.downcast_mut::<T>().unwrap()
    }
}

impl<'res, T: Send + Sync + 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;

    fn retrieve(resource: &RwLock<dyn Any + Send + Sync>) -> Self::Item<'_> {
        ResMut {
            value: resource.write().unwrap(),
            _marker: PhantomData,
        }
    }

    fn get_resource(storage: &Storage) -> ErasedResource {
        storage.get::<T>().unwrap().clone()
    }

    fn type_info() -> Vec<TypeInfo> {
        vec![TypeInfo::new(
            TypeId::of::<T>(),
            type_name::<T>().to_owned(),
        )]
    }
}

// Nested tuple system params
macro_rules! impl_system_param {
    (
        $($params:ident),*
    ) => {
        #[allow(unused)]
        impl<$($params: SystemParam),*> SystemParam for ($($params,)*) {
            type Item<'new> = ($($params::Item<'new>,)*);

            #[allow(clippy::unused_unit)]
            fn retrieve(resource: &RwLock<dyn Any + Send + Sync>) -> Self::Item<'_> {
                ($($params::retrieve(resource),)*)
            }

            fn get_resource(storage: &Storage) -> ErasedResource {
                todo!()
            }


            fn type_info() -> Vec<TypeInfo> {
                let mut out = Vec::new();

                $(
                    out.extend(<$params as SystemParam>::type_info());
                )*

                out
            }
        }
    }
}

impl_system_param!();
impl_system_param!(T1);
impl_system_param!(T1, T2);
impl_system_param!(T1, T2, T3);
impl_system_param!(T1, T2, T3, T4);
impl_system_param!(T1, T2, T3, T4, T5);
impl_system_param!(T1, T2, T3, T4, T5, T6);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8);
