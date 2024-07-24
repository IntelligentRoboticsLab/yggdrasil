use miette::{IntoDiagnostic, Result};
use std::sync::Arc;
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream, ToSocketAddrs,
    },
    task,
};

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

// async fn read_request(stream: Arc<TcpStream>) -> Result<Option<ClientRequest>> {
//     // Store somewhere instead of instatiating
//     let mut msg = [0; 4096];

//     stream.readable().await.into_diagnostic()?;

//     match stream.try_read(&mut msg) {
//         Ok(0) => Ok(None),
//         Ok(num_bytes) => {
//             let client_request: ClientRequest =
//                 bincode::deserialize(&msg[..num_bytes]).into_diagnostic()?;
//             Ok(Some(client_request))
//         }
//         Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Err(miette!("Could not read")),
//         Err(_) => Err(miette!("Something went wrong with reading")),
//     }
// }
