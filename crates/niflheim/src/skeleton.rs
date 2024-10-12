use crate::{InSpace, InverseTransform, Space, SpaceOver, Transform};

pub trait Skeleton {
    fn transform<T1, S1, T2, S2>(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>
    where
        S1: Space + SpaceOver<T1>,
        S2: Space + SpaceOver<T2>,
        Self: Root<T1, S1, T2, S2>
            + Ascend<T1, S1, Self::T, Self::S>
            + Ascend<T2, S2, Self::T, Self::S>,
    {
        self.transform_via(x)
    }

    fn transform_via<T1, S1, T2, S2, T3, S3>(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>
    where
        S1: Space + SpaceOver<T1>,
        S2: Space + SpaceOver<T2>,
        S3: Space + SpaceOver<T3>,
        Self: Ascend<T1, S1, T3, S3> + Ascend<T2, S2, T3, S3>,
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
    T::Transform: Transform<T1, S1, T::Parent> + InverseTransform<T1, S1, T::Parent>,
{
    fn ascend(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2> {
        self.ascend(&self.link().transform(x))
    }

    fn descend(&self, x: &InSpace<T2, S2>) -> InSpace<T1, S1> {
        self.link().inverse_transform(&self.descend(x))
    }
}
