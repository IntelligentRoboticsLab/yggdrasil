//! Inbound buffer for received broadcast messages

use std::collections::VecDeque;
use std::time::Instant;

use super::Message;
use crate::Result;

/// An inbound buffer for receiving messages bundled together in packets.
#[derive(Default)]
pub struct Inbound<A: Clone, M: Message> {
    buffer: VecDeque<(Instant, A, M)>,
}

impl<A: Clone, M: Message> Inbound<A, M> {
    /// Creates a new empty Inbound buffer.
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    /// Removes and returns the oldest message from the buffer.
    pub fn pop(&mut self) -> Option<(Instant, A, M)> {
        self.buffer.pop_front()
    }

    /// Selects and removes a message from the buffer using a selector function.
    ///
    /// Yes you have to copy the fields out of an element that is going to get removed anyway.
    /// Whatever, it's a temporary API until we get events implemented in tyr.
    pub fn take_map<F, T>(&mut self, mut f: F) -> Option<(Instant, A, T)>
    where
        F: FnMut(&Instant, &A, &M) -> Option<T>,
    {
        for i in 0..self.buffer.len() {
            let (when, who, message) = &self.buffer[i];

            if let Some(data) = f(when, who, message) {
                let (when, who, _message) = self.buffer.remove(i).unwrap();
                return Some((when, who, data));
            }
        }

        None
    }

    /// Unpacks a packet of bytes into the buffer at the current time.
    pub fn unpack(&mut self, packet: &[u8], who: A) -> Result<()> {
        self.unpack_at(packet, who, Instant::now())
    }

    /// Unpacks a packet of bytes into the buffer at a specific time.
    pub fn unpack_at(&mut self, mut packet: &[u8], who: A, when: Instant) -> Result<()> {
        while !packet.is_empty() {
            let message = M::decode(&mut packet)?;
            self.buffer.push_back((when, who.clone(), message));
        }

        Ok(())
    }
}
