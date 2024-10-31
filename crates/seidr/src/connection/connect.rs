use async_std::net::{TcpStream, ToSocketAddrs};
use futures::{
    io::{ReadHalf, WriteHalf}, AsyncReadExt
};
use miette::{IntoDiagnostic, Result};
use tokio::time::Duration;

pub struct RobotConnection {
    pub reader: ReadHalf<TcpStream>,
    pub writer: WriteHalf<TcpStream>,
}

impl RobotConnection {
    async fn try_from_ip<A>(addr: A) -> Result<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr).await.into_diagnostic()?;
        let (reader, writer) = stream.split();
        let connection = RobotConnection { reader, writer };
        Ok(connection)
    }

    pub async fn try_connect<A>(addr: A, connection_attempts: i32) -> Result<Self>
    where
        A: ToSocketAddrs + Clone,
    {
        let mut attempt = 0;
        let connection = loop {
            match RobotConnection::try_from_ip(addr.clone()).await {
                Ok(conn) => break conn,
                Err(err) => {
                    tracing::info!(
                        "[{}/{}] Failed to connect: {}. Retrying...",
                        attempt,
                        connection_attempts,
                        err
                    );

                    if attempt >= connection_attempts {
                        tracing::error!("Max connections attempts reached");
                        std::process::exit(1);
                    }

                    attempt += 1;

                    // Wait before retrying
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            };
        };
        Ok(connection)
    }
}