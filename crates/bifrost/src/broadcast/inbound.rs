use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::Message;
use crate::Result;

////////////////////////////////////////////////////////////////////////////////
// Inbound buffer implementation
////////////////////////////////////////////////////////////////////////////////

/// An inbound buffer for receiving messages bundled together in packets.
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
        P: FnMut(Instant, &M) -> Option<T>
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
        let packet = vec![3,3,3,2,2,4,4,4,4];
        let mut buffer: Inbound<Dummy> = Inbound::new();

        let t = Instant::now();

        buffer.unpack_at(&packet, t).unwrap();

        assert_eq!(buffer.pop(), Some((t, Dummy(3))));
        assert_eq!(buffer.pop(), Some((t, Dummy(2))));
        assert_eq!(buffer.pop(), Some((t, Dummy(4))));
        assert_eq!(buffer.pop(), None);
    }
}
