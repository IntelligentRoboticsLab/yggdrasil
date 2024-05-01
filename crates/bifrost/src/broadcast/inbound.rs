use std::collections::VecDeque;
use std::time::Instant;

use super::Message;
use crate::Result;

////////////////////////////////////////////////////////////////////////////////
// Inbound buffer implementation
////////////////////////////////////////////////////////////////////////////////

/// An inbound buffer for receiving messages bundled together in packets.
#[derive(Default)]
pub struct Inbound<M: Message> {
    buffer: VecDeque<(Instant, M)>,
}

impl<M: Message> Inbound<M> {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn pop(&mut self) -> Option<(Instant, M)> {
        self.buffer.pop_front()
    }

    // Yes you have to copy the fields out of an element that is going to get removed anyway.
    // Whatever, it's a temporary API until we get events implemented in tyr.
    pub fn take<P, T>(&mut self, mut selector: P) -> Option<(Instant, T)>
    where
        P: FnMut(Instant, &M) -> Option<T>,
    {
        for i in 0..self.buffer.len() {
            let timestamp = self.buffer[i].0;
            let message = &self.buffer[i].1;

            if let Some(data) = selector(timestamp, message) {
                self.buffer.remove(i);
                return Some((timestamp, data));
            }
        }

        None
    }

    pub fn unpack(&mut self, packet: &[u8]) -> Result<()> {
        self.unpack_at(packet, Instant::now())
    }

    pub fn unpack_at(&mut self, mut packet: &[u8], when: Instant) -> Result<()> {
        while !packet.is_empty() {
            let message = M::decode(&mut packet)?;
            self.buffer.push_back((when, message));
        }

        Ok(())
    }
}
