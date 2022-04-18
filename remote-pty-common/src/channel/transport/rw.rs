use std::{
    io::{Read, Write},
};

use super::Transport;

#[derive(Debug)]
pub struct ReadWriteTransport<R: Read + Send, W: Write + Send> {
    read: R,
    write: W,
}

impl<R: Read + Send, W: Write + Send> ReadWriteTransport<R, W> {
    pub fn new(read: R, write: W) -> Self {
        Self { read, write }
    }
}

impl<R: Read + Send + 'static, W: Write + Send + 'static> Transport for ReadWriteTransport<R, W> {
    fn split(self) -> (Box<dyn Read + Send>, Box<dyn Write + Send>) {
        (Box::new(self.read), Box::new(self.write))
    }
}
