use std::{io::{Read, Write}, str::FromStr, net::SocketAddr};

pub enum TransportType {
    Unix(String),
    Tcp(SocketAddr),
}

// impl FromStr for TransportType {
//     fn from(str: T) -> Self {
        
//     }
// }