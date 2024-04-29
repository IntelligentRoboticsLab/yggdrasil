//! Implementation of primitives to build low-volume broadcasting networks.

pub mod buffer;
pub use buffer::{Buffer, Rate};

use std::time::{Duration, Instant};

use crate::serialization::{Decode, Encode};

////////////////////////////////////////////////////////////////////////////////
// Message definition
////////////////////////////////////////////////////////////////////////////////

/// Trait for messages which can be broadcasted over the network.
///
/// A `Message` may contain anything, provided it does not exceed `MAX_PACKET_SIZE` bytes when
/// encoded. Fragmenting a message into smaller chunks is not (yet) supported and must be done
/// manually.
pub trait Message: Encode + Decode {
    /// The maximum number of bytes a single packet may contain. No message may contain more bytes
    /// but smaller messages may be combined into a single packet.
    const MAX_PACKET_SIZE: usize;
    /// Number of bytes expected to fit one message, used for allocating the encoding buffer.
    const EXPECTED_SIZE: usize;
}

/// Deadline for when a message is to be considered late.
///
/// Note that `Deadline::Within` is relative to when the message is pushed into the buffer, whereas
/// `Deadline::Before` has to have an `Instant` constructed beforehand. Typically, you should use
/// `Deadline::default` unless you have a good reason not to.
#[derive(Debug, Default, Clone, Copy)]
pub enum Deadline {
    #[default]
    Automatic,
    Within(Duration),
    Before(Instant),
}

impl Deadline {
    /// For `Message`s that need to start off as late.
    // omg newjeans reference??
    pub const ASAP: Self = Self::Within(Duration::ZERO);
    /// Literally tomorrow, but can be used to pad out packets with unimportant data.
    pub const WHENEVER: Self = Self::Within(Duration::from_secs(86400));

    pub fn anchor(self, when: Instant) -> Option<Instant> {
        match self {
            Deadline::Automatic => None,
            Deadline::Within(relative) => Some(when + relative),
            Deadline::Before(absolute) => Some(absolute),
        }
    }
}
