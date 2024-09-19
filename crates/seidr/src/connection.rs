use miette::{IntoDiagnostic, Result};
use std::{
    mem,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::{
    io::AsyncReadExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream, ToSocketAddrs,
    },
    task,
};
use yggdrasil::core::control::transmit::RobotStateMsg;

pub struct TcpConnection {
    pub rs: OwnedReadHalf,
    pub ws: Arc<OwnedWriteHalf>,
}

impl TcpConnection {
    pub fn new(reader: OwnedReadHalf, writer: OwnedWriteHalf) -> Self {
        TcpConnection {
            rs: reader,
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

    // pub fn send_request(&self, bytes: Vec<u8>) -> Result<()> {
    //     let ws = self.ws.clone();
    //     task::spawn(async move {
    //         ws.writable().await.into_diagnostic().unwrap();
    //         ws.try_write(bytes.as_slice()).into_diagnostic().unwrap();
    //     });
    //     Ok(())
    // }
}

pub fn send_request(ws: Arc<OwnedWriteHalf>, bytes: Vec<u8>) -> Result<()> {
    // let ws = ws.clone();
    task::spawn(async move {
        ws.writable().await.into_diagnostic().unwrap();
        ws.try_write(bytes.as_slice()).into_diagnostic().unwrap();
    });
    Ok(())
}

pub fn receiving_responses<F>(
    mut rs: OwnedReadHalf,
    last_resource_update: Arc<Mutex<Option<Instant>>>,
    handle_message: F,
) where
    F: Fn(RobotStateMsg) + Send + Sync + 'static,
{
    let mut size_buffer = [0; mem::size_of::<usize>()];
    let mut size: Option<usize> = None;

    tokio::spawn(async move {
        loop {
            rs.readable().await.into_diagnostic().unwrap();

            if let Some(msg_bytes) = size {
                let mut msg = vec![0; msg_bytes];
                rs.read_exact(&mut msg).await.unwrap();
                size = None;

                match bincode::deserialize::<RobotStateMsg>(&msg).into_diagnostic() {
                    Ok(robot_state_msg) => {
                        handle_message(robot_state_msg);
                        *last_resource_update.lock().unwrap() = Some(Instant::now());
                    }
                    Err(e) => {
                        println!("Failed to deserialize server response; err = {:?}", e);
                        break;
                    }
                }
                continue;
            }

            let num_bytes = rs.read(&mut size_buffer).await.unwrap();

            // If the message is zero bytes the connection is closing
            if num_bytes == 0 {
                break;
            }

            if num_bytes == mem::size_of::<usize>() {
                match bincode::deserialize::<usize>(&size_buffer).into_diagnostic() {
                    Ok(size_) => size = Some(size_),
                    Err(e) => {
                        println!(
                            "Failed to deserialize server response, size message; err = {:?}",
                            e
                        );
                        break;
                    }
                }
            }

            // match rs_locked.read(&mut msg_chunk).await.unwrap() {
            //     Ok(0) => break, // Connection closed
            //     Ok(num_bytes) => {
            //         // println!("Bytes received: {}", num_bytes);
            //         // println!("Text received: {}\n", String::from_utf8_lossy(&msg[..num_bytes]));
            //         buffer.extend_from_slice(&msg_chunk[..num_bytes]);

            //         if size.is_none() && buffer.len() >= 4{
            //             let data_size: [u8; 4] = buffer[..4].try_into().expect("Failed to extract size");
            //             size = Some(u32::from_be_bytes(data_size) as usize);
            //             buffer.drain(..4);
            //         }

            //         if let Some(expected_size) = size {
            //             if buffer.len() >= expected_size {

            //             }
            //         }
            //         match bincode::deserialize::<RobotStateMsg>(&msg_chunk[..num_bytes]).into_diagnostic()
            //         {
            //             Ok(robot_state_msg) => {
            //                 handle_message(robot_state_msg);
            //                 *last_resource_update.lock().unwrap() = Some(Instant::now());
            //             }
            //             Err(e) => {
            //                 println!("Failed to deserialize server response; err = {:?}", e);
            //                 println!("Help: {:?}\n", e.downcast::<io::Error>());
            //                 exit(1);
            //                 break;
            //             }
            //         }
            //     }
            //     Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            //     Err(e) => {
            //         println!("Failed to read from socket; err = {:?}", e);
            //         break;
            //     }
            // }
        }
    });
}
