use std::time::Instant;

use super::{Message, Deadline};
use crate::Error;

////////////////////////////////////////////////////////////////////////////////
// Buffer implementation
////////////////////////////////////////////////////////////////////////////////

/// A buffer for sending out messages bundled together in packets.
///
/// A buffer contains a number of fragments and waits until a packet can be adequately filled or a
/// fragment is considered late before sending it out. A fragment is an encoded message together
/// with a deadline.
pub struct Buffer<M: Message>(Vec<Fragment<M>>);

/// Error type returned when adding messages to the buffer.
#[derive(Debug)]
pub enum BufferError {
    TooLong(usize),
    Encoding(Error),
}

impl<M: Message> Buffer<M> {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Pushes a new message into the buffer.
    ///
    /// If it can be expected that an older update still lingers in the buffer, then
    /// `update_or_push` may be used to reduce traffic volume. However, this does require an extra
    /// pass over the entire buffer.
    pub fn push(&mut self, message: M, deadline: Deadline) -> Result<(), BufferError> {
        let fragment = Fragment::new(message, deadline)?;

        let index = match self
            .0
            .iter()
            .rposition(|f| fragment.deadline >= f.deadline)
        {
            Some(pos) => pos + 1,
            None => 0,
        };

        self.0.insert(index, fragment);
        Ok(())
    }

    /// Updates a fragment in the buffer or pushes a new one if none matches `predicate`.
    ///
    /// This function can be used to prevent sending an older update if a new one arrives before it
    /// had been sent out. Since a fragment may be sent out at any time, it cannot be guaranteed
    /// that such an update still exists.
    pub fn update_or_push<P>(&mut self, message: M, deadline: Deadline, mut predicate: P) -> Result<(), BufferError> where 
        P: FnMut(&M) -> bool,
    {
        for fragment in &mut self.0 {
            if predicate(&fragment.message) {
                fragment.update(message, deadline)?;
                return Ok(());
            }
        }

        self.push(message, deadline)
    }

    /// Removes the first matching fragment from the buffer, starting from the end going backwards.
    pub fn remove<P>(&mut self, mut predicate: P) -> Option<M> where P: FnMut(&M) -> bool {
        let index = self.0.iter().rposition(|f| predicate(&f.message))?;
        Some(self.0.remove(index).message)
    }

    /// Clears the entire buffer.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Returns `true` if any of the fragments in the buffer are considered late.
    pub fn late(&self, now: Instant) -> bool {
        match self.0.first() {
            Some(fragment) => fragment.deadline >= now,
            None => false,
        }
    }
}

// It could be more efficient to defer encoding until packing if `update_or_push` becomes the norm.
// When this interface stabilises, maybe make `Fragment` publicly accessible?
struct Fragment<M: Message> {
    deadline: Instant,
    message: M,
    data: Vec<u8>,
}

impl<M: Message> Fragment<M> {
    fn new(message: M, deadline: Deadline) -> Result<Self, BufferError> {
        let mut data = Vec::with_capacity(M::EXPECTED_SIZE);
        message.encode(&mut data).map_err(BufferError::Encoding)?;

        if data.len() > M::MAX_PACKET_SIZE {
            return Err(BufferError::TooLong(data.len()));
        }

        let deadline = match deadline {
            Deadline::Within(duration) => Instant::now() + duration,
            Deadline::Before(instant) => instant,
        };

        Ok(Self { deadline, message, data })
    }

    fn update(&mut self, message: M, deadline: Deadline) -> Result<(), BufferError> {
        let new = Self::new(message, deadline)?;

        if self.deadline > new.deadline {
            self.deadline = new.deadline;
        }
        self.message = new.message;
        self.data = new.data;

        Ok(())
    }

    fn size(&self) -> usize {
        self.data.len()
    }
}
