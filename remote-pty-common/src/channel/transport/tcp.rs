
use std::{net::TcpStream, io::{Read, Write}};

use super::Transport;

#[derive(Debug)]
pub struct TcpTransport {
    socket: TcpStream,
}

impl TcpTransport {
    pub fn new(socket: TcpStream) -> Self {
        Self { socket }
    }
}

impl Transport for TcpTransport {
    fn split(self) -> (Box<dyn Read + Send>, Box<dyn Write + Send>) {
        let reader = self.socket.try_clone().unwrap();
        let writer = self.socket;

        (Box::new(reader), Box::new(writer))
    }
}
