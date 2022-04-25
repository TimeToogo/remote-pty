use std::io::{Read, Write};

pub enum TransportType {
    Unix(String),
    Tcp(SocketAddr),
}

