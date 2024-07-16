use dyn_clone::DynClone;
use miette::{miette, Result, WrapErr};
use std::hash::Hash;
use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::schedule::DependencySystem;
use crate::storage::{ErasedResource, Storage};
use crate::IntoDependencySystem;

use self::private::SystemType;

pub struct NormalSystem;
pub struct StartupSystem;

// Use a sealed trait so we limit the amount of system types
mod private {
    use super::{NormalSystem, StartupSystem};

    pub trait SystemType {}
    impl SystemType for NormalSystem {}
    impl SystemType for StartupSystem {}
}

pub trait System<T: SystemType>: DynClone + Send + Sync + 'static {
    fn run(&mut self, resources: &mut Storage) -> Result<()>;
    fn required_resources(&self) -> Vec<TypeInfo>;
    fn system_type(&self) -> TypeId;
    fn system_name(&self) -> &str;
}

dyn_clone::clone_trait_object!(<T: SystemType> System<T>);

macro_rules! impl_system {
    (
        $($params:ident),*
    ) => {
        #[allow(non_snake_case)]
        #[allow(unused)]
        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> System<NormalSystem> for FunctionSystem<($($params,)*), F>
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

                // I have NO idea why but these need to be separated
                $(
                    let $params = $params::get_resource(&resources).wrap_err_with(|| format!("Failed to get resources in system `{}`", self.system_name()))?;
                )*

                $(
                    let $params = $params::retrieve(&$params);
                )*

                call_inner(&mut self.f, $($params),*)
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
        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> System<StartupSystem> for FunctionSystem<($($params,)*), F>
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
                        let $params = $params::get_resource(&resources)?;
                    )*

                    $(
                        let $params = $params::retrieve(&$params);
                    )*

                    call_inner(&mut self.f, resources, $($params),*)
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
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

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

pub trait IntoSystem<T: SystemType, Input>: Clone {
    type System: System<T>;

    fn into_system(self) -> Self::System;
}

macro_rules! impl_into_system {
    (
        $($params:ident),*
    ) => {
        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> IntoSystem<NormalSystem, ($($params,)*)> for F
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

        impl<F: Clone + Send + Sync + 'static, $($params: SystemParam + 'static),*> IntoSystem<StartupSystem, ($($params,)*)> for F
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
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_into_system!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

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
    type ErasedResources;

    fn retrieve(resource: &Self::ErasedResources) -> Self::Item<'_>;
    fn get_resource(storage: &Storage) -> Result<Self::ErasedResources>;
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
        self.value
            .downcast_ref::<T>()
            .expect("Failed to downcast resource")
    }
}

impl<'res, T: Send + Sync + 'static> SystemParam for Res<'res, T> {
    type Item<'new> = Res<'new, T>;
    type ErasedResources = ErasedResource;

    fn retrieve(resource: &Self::ErasedResources) -> Self::Item<'_> {
        Res {
            value: resource
                .read()
                .expect("Failed to read resource because lock is poisoned!"),
            _marker: PhantomData,
        }
    }

    fn get_resource(storage: &Storage) -> Result<Self::ErasedResources> {
        storage
            .get::<T>()
            .cloned()
            .ok_or_else(|| miette!("Resource `&{}` missing in storage 🤓☝️", type_name::<T>()))
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
        self.value
            .downcast_ref::<T>()
            .expect("Failed to downcast resource")
    }
}

impl<'a, T: Send + Sync + 'static> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
            .downcast_mut::<T>()
            .expect("Failed to downcast resource")
    }
}

impl<'res, T: Send + Sync + 'static> SystemParam for ResMut<'res, T> {
    type Item<'new> = ResMut<'new, T>;
    type ErasedResources = ErasedResource;

    fn retrieve(resource: &Self::ErasedResources) -> Self::Item<'_> {
        ResMut {
            value: resource
                .write()
                .expect("Failed to read resource because lock is poisoned!"),
            _marker: PhantomData,
        }
    }

    fn get_resource(storage: &Storage) -> Result<Self::ErasedResources> {
        storage
            .get::<T>()
            .cloned()
            .ok_or_else(|| miette!("Resource `&mut {}` missing in storage", type_name::<T>()))
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
            type ErasedResources = ($($params::ErasedResources,)*);

            #[allow(clippy::unused_unit)]
            #[allow(non_snake_case)]
            fn retrieve(resource: &Self::ErasedResources) -> Self::Item<'_> {
                let ($($params,)*) = resource;
                ($($params::retrieve($params),)*)
            }

            fn get_resource(storage: &Storage) -> Result<Self::ErasedResources> {
                Ok(($($params::get_resource(storage)?,)*))
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
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_system_param!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

pub trait IntoSystemChain<I> {
    fn chain(self) -> Vec<DependencySystem<()>>;
}

// Implements system chains that run sequentially
macro_rules! impl_system_chain {
    (
        $($systems:ident, $params:ident),*
    ) => {
        #[allow(non_snake_case)]
        impl<$($systems: IntoDependencySystem<$params>, $params),*> IntoSystemChain<($($params),*)> for ($($systems),*) {
            fn chain(self) -> Vec<DependencySystem<()>> {
                // let (T1, T2, ...) = self;
                let ($($systems,)*) = self;

                // IntoDependencySystem<I> -> DependencySystem<I> -> DependencySystem<()>
                $(
                    let $systems: DependencySystem<()> = $systems.into_dependency_system().into_dependency_system();
                )*

                Vec::from([$($systems),*])
            }
        }
    };
}

// every system needs a generic for the system `S*` and its parameters `T*`
// we don't need to chain a single system
// impl_system_chain!(S1, T1);
impl_system_chain!(S1, T1, S2, T2);
impl_system_chain!(S1, T1, S2, T2, S3, T3);
impl_system_chain!(S1, T1, S2, T2, S3, T3, S4, T4);
impl_system_chain!(S1, T1, S2, T2, S3, T3, S4, T4, S5, T5);
impl_system_chain!(S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6);
impl_system_chain!(S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7);
impl_system_chain!(S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8);
impl_system_chain!(S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9);
impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10
);
impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10, S11, T11
);
impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10, S11, T11,
    S12, T12
);
impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10, S11, T11,
    S12, T12, S13, T13
);
impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10, S11, T11,
    S12, T12, S13, T13, S14, T14
);
impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10, S11, T11,
    S12, T12, S13, T13, S14, T14, S15, T15
);

impl_system_chain!(
    S1, T1, S2, T2, S3, T3, S4, T4, S5, T5, S6, T6, S7, T7, S8, T8, S9, T9, S10, T10, S11, T11,
    S12, T12, S13, T13, S14, T14, S15, T15, S16, T16
);
