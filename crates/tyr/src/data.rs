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
        match (self, other) {
            (Self::Shared, Self::Shared) => false,
            (Self::None, _) => false,
            (_, Self::None) => false,
            _ => true,
        }
    }
}

impl Default for AccessMode {
    fn default() -> Self {
        Self::None
    }
}
