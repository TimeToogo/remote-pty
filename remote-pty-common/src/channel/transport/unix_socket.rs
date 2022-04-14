use std::{os::unix::net::UnixStream, io::{Read, Write}};

use super::Transport;

#[derive(Debug)]
pub struct UnixSocketTransport {
    socket: UnixStream,
}

impl UnixSocketTransport {
    pub fn new(socket: UnixStream) -> Self {
        Self { socket }
    }
}

impl Transport for UnixSocketTransport {
    fn split(self) -> (Box<dyn Read + Send>, Box<dyn Write + Send>) {
        let reader = self.socket.try_clone().unwrap();
        let writer = self.socket;

        (Box::new(reader), Box::new(writer))
    }
}
