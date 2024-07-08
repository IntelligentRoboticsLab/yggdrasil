//! Outbound buffer for pending broadcast messages

use std::{
    fmt,
    time::{Duration, Instant},
};

use super::{Deadline, Message};
use crate::Error;

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

/// A struct for specifying the rate at which an `Outbound` releases new packets.
pub struct Rate {
    /// The minimum interval before the next late packet may be sent out.
    pub late_threshold: Duration,
    /// The automatic deadline applied to fragments using `Deadline::Automatic`.
    pub automatic_deadline: Duration,
    /// The minimum interval before the next early packet may be sent out.
    pub early_threshold: Duration,
}

/// Error type returned when adding messages to the buffer.
#[derive(Debug)]
pub enum OutboundError {
    /// The message is too long to fit inside a packet.
    TooLong(usize),
    /// Error encountered during message encoding.
    Encoding(Error),
}

impl fmt::Display for OutboundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong(size) => write!(f, "{} is too long to fit in a single packet.", size),
            Self::Encoding(error) => write!(f, "{}", error),
        }
    }
}

impl std::error::Error for OutboundError {}

impl<M: Message> Outbound<M> {
    /// Creates an empty buffer with the given rate configuration.
    pub fn new(rate: Rate) -> Self {
        Self {
            fragments: Vec::new(),
            last: Instant::now(),
            rate,
        }
    }

    /// Pushes a message into the buffer registered at the current time.
    ///
    /// If it can be expected that an older update still lingers in the buffer, then
    /// [`Outbound::update_or_push`] may be used to reduce traffic volume. However, this does require an extra
    /// pass over the entire buffer.
    pub fn push(&mut self, message: M) -> Result<(), OutboundError> {
        self.push_at(message, Deadline::default(), Instant::now())
    }

    /// Pushes a message into the buffer registered at the current time, to be delivered by the given deadline.
    pub fn push_by(&mut self, message: M, deadline: Deadline) -> Result<(), OutboundError> {
        self.push_at(message, deadline, Instant::now())
    }

    /// Pushes a message into the buffer registered at the specified time, to be delivered by the given deadline.
    pub fn push_at(
        &mut self,
        message: M,
        deadline: Deadline,
        when: Instant,
    ) -> Result<(), OutboundError> {
        let deadline = deadline
            .absolute(when)
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
    pub fn update_or_push(&mut self, message: M) -> Result<(), OutboundError> {
        self.update_or_push_at(message, Deadline::default(), Instant::now())
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found,
    /// using the provided deadline.
    pub fn update_or_push_by(
        &mut self,
        message: M,
        deadline: Deadline,
    ) -> Result<(), OutboundError> {
        self.update_or_push_at(message, deadline, Instant::now())
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found,
    /// at the given time.
    pub fn update_or_push_at(
        &mut self,
        mut message: M,
        deadline: Deadline,
        when: Instant,
    ) -> Result<(), OutboundError> {
        for fragment in &mut self.fragments {
            if message.try_merge(&fragment.message) {
                return fragment.update(message);
            }
        }

        self.push_at(message, deadline, when)
    }

    /// Packs the fragments in the buffer into a single packet at the current time.
    pub fn try_pack(&mut self) -> Option<Vec<u8>> {
        self.try_pack_at(Instant::now())
    }

    /// Packs the fragments in the buffer into a single packet at the given time.
    pub fn try_pack_at(&mut self, when: Instant) -> Option<Vec<u8>> {
        // If we are late or we should send early packets, pack a new packet.
        if self.late(when) || self.early(when) {
            self.last = when;
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
    ///
    /// An early packet gets send out if the buffer is (almost) full but no deadlines have yet
    /// finished.
    fn early(&self, when: Instant) -> bool {
        if when.duration_since(self.last) < self.rate.early_threshold {
            return false;
        }

        self.underfullness() <= M::DEAD_SPACE
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
