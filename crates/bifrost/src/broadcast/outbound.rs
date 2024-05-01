use std::time::{Duration, Instant};

use super::{Deadline, Message};
use crate::Error;

////////////////////////////////////////////////////////////////////////////////
// Outbound buffer implementation
////////////////////////////////////////////////////////////////////////////////

/// An outbound buffer for sending out messages bundled together in packets.
///
/// An outbound buffer contains a number of fragments and waits until a packet can be adequately filled or a
/// fragment is considered late before sending it out. A fragment is an encoded message together
/// with a deadline.
pub struct Outbound<M: Message> {
    /// Fragments waiting to be sent out.
    fragments: Vec<Fragment<M>>,
    /// Last time a packet has been packed.
    last: Instant,
    /// The rate configuration for the buffer, controlling timing thresholds.
    pub rate: Rate,
}

/// A struct for specifying the rate at which a `Outbound` releases new packets.
pub struct Rate {
    /// The minimum interval before the next late packet may be sent out.
    pub late_threshold: Duration,
    /// The automatic deadline applied to fragments using `Deadline::Automatic`.
    pub automatic_deadline: Duration,
    /// The minimum interval before the next early packet may be sent out.
    pub early_threshold: Duration,
    /// Number of bytes allowed to remain before a packet is considered full enough.
    pub dead_space: usize,
}

/// Error type returned when adding messages to the buffer.
#[derive(Debug)]
pub enum OutboundError {
    /// The message is too long to fit inside a packet.
    TooLong(usize),
    /// Error encountered during message encoding.
    Encoding(Error),
}

impl<M: Message> Outbound<M> {
    /// Creates an empty buffer with the given rate configuration.
    pub fn new(rate: Rate) -> Self {
        Self {
            fragments: Vec::new(),
            last: Instant::now(),
            rate,
        }
    }

    /// Pushes a message into the buffer with the current time as anchor.
    ///
    /// If it can be expected that an older update still lingers in the buffer, then
    /// [`Outbound::update_or_push`] may be used to reduce traffic volume. However, this does require an extra
    /// pass over the entire buffer.
    pub fn push(&mut self, message: M) -> Result<(), OutboundError> {
        self.push_at(message, Deadline::default(), Instant::now())
    }

    /// Pushes a message into the buffer with the current time as anchor, to be delivered by the given deadline.
    pub fn push_by(&mut self, message: M, deadline: Deadline) -> Result<(), OutboundError> {
        self.push_at(message, deadline, Instant::now())
    }

    /// Pushes a message into the buffer with the given time as anchor, to be delivered by the given deadline.
    pub fn push_at(
        &mut self,
        message: M,
        deadline: Deadline,
        when: Instant,
    ) -> Result<(), OutboundError> {
        let deadline = deadline
            .anchor(when)
            .unwrap_or_else(|| when + self.rate.automatic_deadline);

        let fragment = Fragment::new(message, deadline)?;

        let index = match self
            .fragments
            .iter()
            .rposition(|f| fragment.deadline >= f.deadline)
        {
            Some(pos) => pos + 1,
            None => 0,
        };

        self.fragments.insert(index, fragment);
        Ok(())
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found.
    ///
    /// This function can be used to prevent sending an older update if a new one arrives before it
    /// had been sent out. Since a fragment may be sent out at any time, it cannot be guaranteed
    /// that such an update still exists.
    pub fn update_or_push<P>(&mut self, message: M, predicate: P) -> Result<(), OutboundError>
    where
        P: FnMut(&M) -> bool,
    {
        self.update_or_push_at(message, Deadline::default(), Instant::now(), predicate)
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found,
    /// using the provided deadline.
    pub fn update_or_push_by<P>(
        &mut self,
        message: M,
        deadline: Deadline,
        predicate: P,
    ) -> Result<(), OutboundError>
    where
        P: FnMut(&M) -> bool,
    {
        self.update_or_push_at(message, deadline, Instant::now(), predicate)
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found,
    /// at the given time.
    pub fn update_or_push_at<P>(
        &mut self,
        message: M,
        deadline: Deadline,
        when: Instant,
        mut predicate: P,
    ) -> Result<(), OutboundError>
    where
        P: FnMut(&M) -> bool,
    {
        for fragment in &mut self.fragments {
            if predicate(&fragment.message) {
                return fragment.update(message);
            }
        }

        self.push_at(message, deadline, when)
    }

    /// Packs the fragments in the buffer into a single packet at the current time.
    pub fn pack(&mut self) -> Option<Vec<u8>> {
        self.pack_at(Instant::now())
    }

    /// Packs the fragments in the buffer into a single packet at the given time.
    pub fn pack_at(&mut self, when: Instant) -> Option<Vec<u8>> {
        // If we are late or we should send early packets, pack a new packet.
        if self.late(when) || self.early(when) {
            Some(self.do_pack())
        } else {
            None
        }
    }

    /// Packs the fragments in the buffer into a packet without rate control.
    fn do_pack(&mut self) -> Vec<u8> {
        // We know the upper limit on packet size, no need for multiple allocations.
        let mut remaining = M::MAX_PACKET_SIZE;
        let mut packet = Vec::with_capacity(remaining);

        // We shouldn't have to step through the rest of the buffer if the packet is already full,
        // but `Vec::retain` allows for some clean code.
        self.fragments.retain(|f| {
            if f.size() <= remaining {
                remaining -= f.size();
                packet.extend_from_slice(&f.data);
                false
            } else {
                true
            }
        });

        packet
    }

    /// Counts the the remaining number of bytes if we were to pack a single packet.
    fn underfullness(&self) -> usize {
        let mut remaining = M::MAX_PACKET_SIZE;

        for f in &self.fragments {
            if f.size() <= remaining {
                remaining -= f.size();
            }
        }

        remaining
    }

    /// Checks if we need to send out a late packet.
    fn late(&self, when: Instant) -> bool {
        if when.duration_since(self.last) < self.rate.late_threshold {
            return false;
        }

        match self.fragments.first() {
            Some(first) => when >= first.deadline,
            None => false,
        }
    }

    /// Checks if we need to send out an early packet.
    fn early(&self, when: Instant) -> bool {
        if when.duration_since(self.last) < self.rate.early_threshold {
            return false;
        }

        self.underfullness() <= self.rate.dead_space
    }
}

/// A fragment of a message in the buffer.
///
/// Contains the encoded data, the original message, and its deadline.
// It could be more efficient to defer encoding until packing if `update_or_push` becomes the norm.
// When this interface stabilises, maybe make `Fragment` publicly accessible?
struct Fragment<M: Message> {
    message: M,
    data: Vec<u8>,
    deadline: Instant,
}

impl<M: Message> Fragment<M> {
    /// Creates a new fragment with the given message and deadline.
    fn new(message: M, deadline: Instant) -> Result<Self, OutboundError> {
        let data = Self::encode(&message)?;

        Ok(Self {
            message,
            data,
            deadline,
        })
    }

    /// Updates the fragment with a new message.
    fn update(&mut self, message: M) -> Result<(), OutboundError> {
        self.data = Self::encode(&message)?;
        self.message = message;

        Ok(())
    }

    /// Encodes the message into bytes.
    fn encode(message: &M) -> Result<Vec<u8>, OutboundError> {
        let mut data = Vec::with_capacity(M::EXPECTED_SIZE);
        message.encode(&mut data).map_err(OutboundError::Encoding)?;

        if data.len() > M::MAX_PACKET_SIZE {
            return Err(OutboundError::TooLong(data.len()));
        }

        Ok(data)
    }

    /// Returns the size of the fragment data.
    fn size(&self) -> usize {
        self.data.len()
    }
}

////////////////////////////////////////////////////////////////////////////////
// Unit tests
////////////////////////////////////////////////////////////////////////////////

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
    fn test_outbound() {
        let rate = Rate {
            late_threshold: Duration::ZERO,
            automatic_deadline: Duration::from_secs(5),
            early_threshold: Duration::from_secs(10),
            dead_space: 2,
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

        assert_eq!(buffer.pack_at(t + 5), Some(vec![1, 7, 7, 7, 7, 7, 7, 7]));
        assert_eq!(buffer.pack_at(t + 6), Some(vec![4, 4, 4, 4, 3, 3, 3]));
        assert_eq!(buffer.pack_at(t + 7), Some(vec![6, 6, 6, 6, 6, 6]));
    }

    #[test]
    fn test_outbound_early() {
        let rate = Rate {
            late_threshold: Duration::ZERO,
            automatic_deadline: Duration::ZERO,
            early_threshold: Duration::ZERO,
            dead_space: 2,
        };

        let mut buffer = Outbound::new(rate);
        let t = Epoch(Instant::now());

        buffer.push_at(Dummy(3), Deadline::WHENEVER, t + 0).unwrap();
        assert_eq!(buffer.pack_at(t + 0), None);
        buffer.push_at(Dummy(2), Deadline::WHENEVER, t + 1).unwrap();
        assert_eq!(buffer.pack_at(t + 1), None);
        buffer.push_at(Dummy(1), Deadline::WHENEVER, t + 2).unwrap();
        assert_eq!(buffer.pack_at(t + 2), Some(vec![3, 3, 3, 2, 2, 1]));
    }
}
