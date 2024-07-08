//! Implementation of primitives to build low-volume broadcasting networks.

pub mod inbound;
pub mod outbound;

pub use inbound::Inbound;
pub use outbound::{Outbound, Rate};

use std::time::{Duration, Instant};

use crate::serialization::{Decode, Encode};

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
    /// Number of bytes allowed to remain before a packet is considered full enough.
    const DEAD_SPACE: usize;

    /// Returns true if `self` is an update of `old`. `self` is mutable to allow for merging the
    /// old message into the new one.
    fn try_merge(&mut self, old: &Self) -> bool {
        let _ = old;
        false
    }
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

    pub fn absolute(self, when: Instant) -> Option<Instant> {
        match self {
            Deadline::Automatic => None,
            Deadline::Within(relative) => Some(when + relative),
            Deadline::Before(absolute) => Some(absolute),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::ops::Add;

    use super::*;
    use crate::{
        serialization::{Decode, Encode},
        Result,
    };

    /// Dummy message that encodes to `n` bytes with value `n`.
    #[derive(Debug, PartialEq, Eq)]
    struct Dummy(u8);

    impl Encode for Dummy {
        fn encode(&self, mut write: impl Write) -> Result<()> {
            Ok(write.write_all(&vec![self.0; self.encode_len()])?)
        }

        fn encode_len(&self) -> usize {
            self.0 as usize
        }
    }

    impl Decode for Dummy {
        fn decode(mut read: impl Read) -> Result<Self> {
            let mut buf = [0; Self::MAX_PACKET_SIZE];
            read.read_exact(&mut buf[..1])?;
            let n = buf[0] as usize;
            read.read_exact(&mut buf[1..n])?;
            Ok(Self(buf[0]))
        }
    }

    impl Message for Dummy {
        const MAX_PACKET_SIZE: usize = 8;
        const EXPECTED_SIZE: usize = 8;
        const DEAD_SPACE: usize = 2;
    }

    /// Helper type for quickly constructing timestamps.
    #[derive(Clone, Copy)]
    struct Epoch(Instant);

    impl Add<isize> for Epoch {
        type Output = Instant;

        fn add(self, rhs: isize) -> Instant {
            if rhs >= 0 {
                self.0 + Duration::from_secs(rhs as u64)
            } else {
                self.0 - Duration::from_secs(-rhs as u64)
            }
        }
    }

    #[test]
    fn test_inbound() {
        let packet = vec![3, 3, 3, 2, 2, 4, 4, 4, 4];
        let mut buffer = Inbound::new();

        let t = Instant::now();

        buffer.unpack_at(&packet, (), t).unwrap();

        assert_eq!(buffer.pop(), Some((t, (), Dummy(3))));
        assert_eq!(buffer.pop(), Some((t, (), Dummy(2))));
        assert_eq!(buffer.pop(), Some((t, (), Dummy(4))));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_outbound() {
        let rate = Rate {
            late_threshold: Duration::ZERO,
            automatic_deadline: Duration::from_secs(5),
            early_threshold: Duration::from_secs(10),
        };

        let mut buffer = Outbound::new(rate);
        let t = Epoch(Instant::now());

        buffer
            .push_at(Dummy(7), Deadline::Automatic, t + 0)
            .unwrap();
        buffer
            .push_at(Dummy(4), Deadline::Automatic, t + 1)
            .unwrap();
        buffer
            .push_at(Dummy(6), Deadline::Automatic, t + 2)
            .unwrap();
        buffer
            .push_at(Dummy(3), Deadline::Automatic, t + 3)
            .unwrap();
        buffer.push_at(Dummy(1), Deadline::ASAP, t + 4).unwrap();

        assert_eq!(
            buffer.try_pack_at(t + 5),
            Some(vec![1, 7, 7, 7, 7, 7, 7, 7])
        );
        assert_eq!(buffer.try_pack_at(t + 6), Some(vec![4, 4, 4, 4, 3, 3, 3]));
        assert_eq!(buffer.try_pack_at(t + 7), Some(vec![6, 6, 6, 6, 6, 6]));
    }

    #[test]
    fn test_outbound_early() {
        let rate = Rate {
            late_threshold: Duration::ZERO,
            automatic_deadline: Duration::ZERO,
            early_threshold: Duration::ZERO,
        };

        let mut buffer = Outbound::new(rate);
        let t = Epoch(Instant::now());

        buffer.push_at(Dummy(3), Deadline::WHENEVER, t + 0).unwrap();
        assert_eq!(buffer.try_pack_at(t + 0), None);
        buffer.push_at(Dummy(2), Deadline::WHENEVER, t + 1).unwrap();
        assert_eq!(buffer.try_pack_at(t + 1), None);
        buffer.push_at(Dummy(1), Deadline::WHENEVER, t + 2).unwrap();
        assert_eq!(buffer.try_pack_at(t + 2), Some(vec![3, 3, 3, 2, 2, 1]));
    }
}
