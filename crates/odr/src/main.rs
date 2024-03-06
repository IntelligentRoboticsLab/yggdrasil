use std::io;

use std::time::Duration;

use nidhogg::{
    backend::{ConnectWithRetry, LolaBackend, ReadHardwareInfo}, types::JointArray, HardwareInfo, NaoBackend, NaoControlMessage, NaoState, types::FillExt
};


// I have setup a connection to Lola, however it it necessary to write a message to the backend very 12ms. 
// The write call to the backend end is blocking. So if you make a loop and write in it every iteration, that should work.
// The difficult thing wil be, reading user input, but not blocking on it and still making sure messages are sent. 

fn main() -> miette::Result<()> {
    let mut nao = LolaBackend::connect_with_retry(10, Duration::from_millis(500))?;


    // Example of reading a msg
    let a = nao.read_nao_state()?;
    a.position; // Into json

    // Example of sending a msg
    let msg = NaoControlMessage::builder().position(JointArray::<f32>::fill(0.0)).build();
    let b = nao.send_control_msg(msg)?;

    println!("Hello, world!");
    let mut buffer = String::new();
    while io::stdin().read_line(&mut buffer).unwrap() > 0 {
        println!("{}", buffer);
    }

    Ok(())
}
