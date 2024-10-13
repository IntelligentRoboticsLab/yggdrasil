use crate::{InSpace, InverseTransform, Space, SpaceOver, Transform};

#[macro_export]
macro_rules! parent {
    ($Skeleton:ty, $Space:ty) => {
        impl<T: Clone> ::niflheim::skeleton::Ascend<T, $Space, T, $Space> for $Skeleton
        where
            $Space: SpaceOver<T>,
        {
            fn ascend(&self, x: &::niflheim::InSpace<T, $Space>) -> ::niflheim::InSpace<T, $Space> {
                x.clone()
            }
        }

        impl<T: Clone> ::niflheim::skeleton::Descend<T, $Space, T, $Space> for $Skeleton
        where
            $Space: SpaceOver<T>,
        {
            fn descend(
                &self,
                x: &::niflheim::InSpace<T, $Space>,
            ) -> ::niflheim::InSpace<T, $Space> {
                x.clone()
            }
        }
    };
}

#[macro_export]
macro_rules! link {
    ($Skeleton:ty, $Transform:ty, $Child:ty, $Parent:ty, $field:ident) => {
        impl ::niflheim::skeleton::Link<$Child, $Parent> for $Skeleton {
            type Parent = $Parent;
            type Transform = ::niflheim::BetweenSpaces<$Transform, $Child, $Parent>;

            fn link(&self) -> &Self::Transform {
                &self.$field
            }
        }
    };
}

#[macro_export]
macro_rules! propagate_link {
    ($Skeleton:ty, $Child:ty, $Parent:ty) => {
        impl<C: Space> ::niflheim::skeleton::Link<C, $Parent> for $Skeleton
        where
            Self: ::niflheim::skeleton::Link<C, $Child>,
        {
            type Parent = <Self as ::niflheim::skeleton::Link<C, $Child>>::Parent;
            type Transform = <Self as ::niflheim::skeleton::Link<C, $Child>>::Transform;

            fn link(&self) -> &Self::Transform {
                self.link()
            }
        }
    };
}

pub trait Skeleton {
    fn transform<T1, S1, T2, S2>(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>
    where
        S1: Space + SpaceOver<T1>,
        S2: Space + SpaceOver<T2>,
        Self: Root<T1, S1, T2, S2>
            + Ascend<T1, S1, Self::T, Self::S>
            + Descend<T2, S2, Self::T, Self::S>,
    {
        self.transform_via(x)
    }

    fn transform_via<T1, S1, T2, S2, T3, S3>(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>
    where
        S1: Space + SpaceOver<T1>,
        S2: Space + SpaceOver<T2>,
        S3: Space + SpaceOver<T3>,
        Self: Ascend<T1, S1, T3, S3> + Descend<T2, S2, T3, S3>,
    {
        self.descend(&self.ascend(x))
    }
}

pub trait Root<T1, S1, T2, S2> {
    type T;
    type S: Space + SpaceOver<Self::T>;
}

pub trait Link<S1: Space, S2: Space>: Skeleton {
    type Parent: Space;
    type Transform;

    fn link(&self) -> &Self::Transform;
}

pub trait Ascend<T1, S1, T2, S2>: Skeleton
where
    S1: Space + SpaceOver<T1>,
    S2: Space + SpaceOver<T2>,
{
    fn ascend(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>;
}

pub trait Descend<T1, S1, T2, S2>: Skeleton
where
    S1: Space + SpaceOver<T1>,
    S2: Space + SpaceOver<T2>,
{
    fn descend(&self, x: &InSpace<T2, S2>) -> InSpace<T1, S1>;
}

type TransformOutput<T, T1, S1, S2> =
    <<T as Link<S1, S2>>::Transform as Transform<T1, S1, <T as Link<S1, S2>>::Parent>>::Output;

impl<T, T1, S1, T2, S2> Ascend<T1, S1, T2, S2> for T
where
    S1: Space + SpaceOver<T1>,
    S2: Space + SpaceOver<T2>,
    T: Link<S1, S2> + Ascend<TransformOutput<T, T1, S1, S2>, T::Parent, T2, S2>,
    T::Parent: SpaceOver<TransformOutput<T, T1, S1, S2>>,
    T::Transform: Transform<T1, S1, T::Parent>,
{
    fn ascend(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2> {
        self.ascend(&self.link().transform(x))
    }
}

impl<T, T1, S1, T2, S2> Descend<T1, S1, T2, S2> for T
where
    S1: Space + SpaceOver<T1>,
    S2: Space + SpaceOver<T2>,
    T: Link<S1, S2> + Descend<TransformOutput<T, T1, S1, S2>, T::Parent, T2, S2>,
    T::Parent: SpaceOver<TransformOutput<T, T1, S1, S2>>,
    T::Transform: InverseTransform<T1, S1, T::Parent>,
{
    fn descend(&self, x: &InSpace<T2, S2>) -> InSpace<T1, S1> {
        self.link().inverse_transform(&self.descend(x))
    }
}
