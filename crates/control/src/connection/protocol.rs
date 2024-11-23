use std::{collections::HashMap, fmt::Debug};

use bifrost::serialization::{Decode, Encode};
use uuid::Uuid;

use crate::debug_system::DebugEnabledSystems;

pub const CONTROL_PORT: u16 = 40001;

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

// pub trait MessageEncodable {
//     fn encode(&self) -> io::Result<Vec<u8>>;
//     fn encode_into(&self, buffer: &mut [u8]) -> io::Result<()>;
// }

// pub trait MessageDecodable: Sized {
//     fn decode(data: &[u8]) -> io::Result<Self>;
// }

// impl<T> MessageEncodable for T
// where
//     T: Serialize,
// {
//     fn encode(&self) -> io::Result<Vec<u8>> {
//         bincode::serialize(self).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
//     }

//     fn encode_into(&self, buffer: &mut [u8]) -> io::Result<()> {
//         bincode::serialize_into(buffer, self).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
//     }
// }

// impl<T> MessageDecodable for T
// where
//     T: for<'de> Deserialize<'de>,
// {
//     fn decode(data: &[u8]) -> io::Result<Self> {
//         bincode::deserialize(data).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))
//     }
// }

// pub trait Message: MessageEncodable + MessageDecodable + Send + Clone + Debug {
//     fn is_disconnected(&self) -> bool;
// }


#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    Disconnect,
    Resources(HashMap<String, String>),
    DebugEnabledSystems(DebugEnabledSystems),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerMessage {
    Disconnect,
    Connected(Uuid),
    UpdateResource(String, String),
    SendResourcesNow,
    UpdateEnabledDebugSystem(String, bool),
}


// impl Message for ViewerMessage {
//     fn is_disconnected(&self) -> bool {
//         matches!(self, ViewerMessage::Disconnect)
//     }
// }
