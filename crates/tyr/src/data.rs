pub use tyr_macros::Data;

pub trait Data {
    type Access: Access;
}

pub trait Access: Default {
    fn conflicts_with(&self, other: &Self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessMode {
    None,
    Shared,
    Exclusive,
}

impl AccessMode {
    pub fn conflicts_with(self, other: Self) -> bool {
        !matches!(
            (self, other),
            (Self::Shared, Self::Shared) | (Self::None, _) | (_, Self::None)
        )
    }
}

impl Default for AccessMode {
    fn default() -> Self {
        Self::None
    }
}
