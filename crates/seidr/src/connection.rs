use miette::{IntoDiagnostic, Result};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::{
    io,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream, ToSocketAddrs,
    },
    task,
};
use yggdrasil::core::control::transmit::RobotStateMsg;

const BUFFER_SIZE: usize = 4096;

pub struct TcpConnection {
    pub rs: Arc<OwnedReadHalf>,
    pub ws: Arc<OwnedWriteHalf>,
}

impl TcpConnection {
    pub fn new(reader: OwnedReadHalf, writer: OwnedWriteHalf) -> Self {
        TcpConnection {
            rs: Arc::new(reader),
            ws: Arc::new(writer),
        }
    }

    pub async fn try_from_ip<A>(addr: A) -> Result<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr).await.into_diagnostic()?;
        let (rs, ws) = stream.into_split();
        let connection = TcpConnection::new(rs, ws);
        Ok(connection)
    }

    pub fn send_request(&self, bytes: Vec<u8>) -> Result<()> {
        let ws = self.ws.clone();
        task::spawn(async move {
            ws.writable().await.into_diagnostic().unwrap();
            ws.try_write(bytes.as_slice()).into_diagnostic().unwrap();
        });
        Ok(())
    }
}

pub fn receiving_responses<F>(
    rs: Arc<OwnedReadHalf>,
    last_resource_update: Arc<Mutex<Option<Instant>>>,
    handle_message: F,
) where
    F: Fn(RobotStateMsg) + Send + Sync + 'static,
{
    let mut msg = [0; BUFFER_SIZE];
    tokio::spawn(async move {
        loop {
            rs.readable().await.into_diagnostic().unwrap();
            match rs.try_read(&mut msg) {
                Ok(0) => break, // Connection closed
                Ok(num_bytes) => {
                    if num_bytes >= BUFFER_SIZE {
                        println!("Buffer size might be to small: {num_bytes} received, Buffer size: {BUFFER_SIZE}");
                    }
                    match bincode::deserialize::<RobotStateMsg>(&msg[..num_bytes]).into_diagnostic()
                    {
                        Ok(robot_state_msg) => {
                            handle_message(robot_state_msg);
                            *last_resource_update.lock().unwrap() = Some(Instant::now());
                        }
                        Err(e) => {
                            println!("Failed to deserialize server response; err = {:?}", e);
                            break;
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    println!("Failed to read from socket; err = {:?}", e);
                    break;
                }
            }
        }
    });
}
