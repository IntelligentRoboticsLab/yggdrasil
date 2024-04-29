use std::time::{Duration, Instant};

use super::{Deadline, Message};
use crate::Error;

////////////////////////////////////////////////////////////////////////////////
// Buffer implementation
////////////////////////////////////////////////////////////////////////////////

/// A buffer for sending out messages bundled together in packets.
///
/// A buffer contains a number of fragments and waits until a packet can be adequately filled or a
/// fragment is considered late before sending it out. A fragment is an encoded message together
/// with a deadline.
pub struct Buffer<M: Message> {
    fragments: Vec<Fragment<M>>,
    /// The rate configuration for the buffer, controlling timing thresholds.
    pub rate: Rate,
}

/// A struct for specifying the rate at which a `Buffer` releases new packets.
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
pub enum BufferError {
    /// The message is too long to fit inside a packet.
    TooLong(usize),
    /// Error encountered during message encoding.
    Encoding(Error),
}

impl<M: Message> Buffer<M> {
    /// Creates an empty buffer with the given rate configuration.
    pub fn new(rate: Rate) -> Self {
        Self { fragments: Vec::new(), rate }
    }

    /// Pushes a message into the buffer with the current time as anchor.
    pub fn push(&mut self, message: M) -> Result<(), BufferError> {
        self.push_at(message, Deadline::default(), Instant::now())
    }

    /// Pushes a message into the buffer with the current time as anchor, to be delivered by the given deadline.
    pub fn push_by(&mut self, message: M, deadline: Deadline) -> Result<(), BufferError> {
        self.push_at(message, deadline, Instant::now())
    }

    /// Pushes a message into the buffer with the given time as anchor, to be delivered by the given deadline.
    ///
    /// If it can be expected that an older update still lingers in the buffer, then
    /// `update_or_push` may be used to reduce traffic volume. However, this does require an extra
    /// pass over the entire buffer.
    pub fn push_at(
        &mut self,
        message: M,
        deadline: Deadline,
        when: Instant,
    ) -> Result<(), BufferError> {
        let deadline = deadline
            .anchor(when)
            .unwrap_or_else(|| when + self.rate.automatic_deadline);

        let fragment = Fragment::new(message, deadline)?;

        let index = match self.fragments.iter().rposition(|f| fragment.deadline >= f.deadline) {
            Some(pos) => pos + 1,
            None => 0,
        };

        self.fragments.insert(index, fragment);
        Ok(())
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found.
    pub fn update_or_push<P>(&mut self, message: M, predicate: P) -> Result<(), BufferError>
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
    ) -> Result<(), BufferError>
    where
        P: FnMut(&M) -> bool,
    {
        self.update_or_push_at(message, deadline, Instant::now(), predicate)
    }

    /// Updates a message in the buffer according to the given predicate or pushes it if not found,
    /// at the given time.
    ///
    /// This function can be used to prevent sending an older update if a new one arrives before it
    /// had been sent out. Since a fragment may be sent out at any time, it cannot be guaranteed
    /// that such an update still exists.
    pub fn update_or_push_at<P>(
        &mut self,
        message: M,
        deadline: Deadline,
        when: Instant,
        mut predicate: P,
    ) -> Result<(), BufferError>
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
        unimplemented!() // TODO: Implement packing logic
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
    fn new(message: M, deadline: Instant) -> Result<Self, BufferError> {
        let data = Self::encode(&message)?;

        Ok(Self { message, data, deadline })
    }

    /// Updates the fragment with a new message.
    fn update(&mut self, message: M) -> Result<(), BufferError> {
        self.data = Self::encode(&message)?;
        self.message = message;

        Ok(())
    }

    /// Encodes the message into bytes.
    fn encode(message: &M) -> Result<Vec<u8>, BufferError> {
        let mut data = Vec::with_capacity(M::EXPECTED_SIZE);
        message.encode(&mut data).map_err(BufferError::Encoding)?;

        if data.len() > M::MAX_PACKET_SIZE {
            return Err(BufferError::TooLong(data.len()));
        }

        Ok(data)
    }

    /// Returns the size of the fragment data.
    fn size(&self) -> usize {
        self.data.len()
    }
}

