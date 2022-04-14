use std::io::{Read, Write};

// generic duplex transport interface
pub trait Transport {
    fn split(self) -> (Box<dyn Read + Send>, Box<dyn Write + Send>);
}

pub mod unix_socket;
pub mod tcp;
pub mod mem;

