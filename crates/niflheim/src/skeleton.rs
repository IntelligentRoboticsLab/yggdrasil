#![allow(dead_code)]
#![allow(unused_imports)]

use crate::{InSpace, InverseTransform, Space, SpaceOver, Transform};

pub trait Link<A, B>
where 
    A: Space,
    B: Space,
    Self::Parent: Space,
{
    type Parent;
    type Transform;

    fn link(&self) -> &Self::Transform;
}

pub trait Ascend<T1, S1, T2, S2>
where 
    S1: Space + SpaceOver<T1>,
    S2: Space + SpaceOver<T2>,
{
    fn ascend(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>;
    fn descend(&self, x: &InSpace<T2, S2>) -> InSpace<T1, S1>;
}

impl<G, T1, S1, T2, S2> Ascend<T1, S1, T2, S2> for G
where 
    S1: Space + SpaceOver<T1>,
    S2: Space + SpaceOver<T2>,
    G: Link<S1, S2> + Ascend<<G::Transform as Transform<T1, S1, G::Parent>>::Output, G::Parent, T2, S2>,
    G::Parent: SpaceOver<<G::Transform as Transform<T1, S1, G::Parent>>::Output>,
    G::Transform: Transform<T1, S1, G::Parent> + InverseTransform<T1, S1, G::Parent>,
{
    fn ascend(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2> {
        self.ascend(&self.link().transform(x))
    }

    fn descend(&self, x: &InSpace<T2, S2>) -> InSpace<T1, S1> {
        self.link().inverse_transform(&self.descend(x))
    }
}
