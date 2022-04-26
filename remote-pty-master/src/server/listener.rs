use std::{io::Result, net::TcpListener, os::unix::net::UnixListener};

use remote_pty_common::channel::{
    transport::{conf::TransportType, tcp::TcpTransport, unix_socket::UnixSocketTransport},
    RemoteChannel,
};

// generic listener interface to accept incoming connections to the server
pub trait Listener {
    fn accept(&mut self) -> Result<RemoteChannel>;
}

pub struct UnixSocketListener {
    listener: UnixListener,
}

impl UnixSocketListener {
    pub fn new(listener: UnixListener) -> Self {
        Self { listener }
    }
}

impl Listener for UnixSocketListener {
    fn accept(&mut self) -> Result<RemoteChannel> {
        let (socket, _) = self.listener.accept()?;

        Ok(RemoteChannel::new(UnixSocketTransport::new(socket)))
    }
}

pub struct TcpSocketListener {
    listener: TcpListener,
}

impl TcpSocketListener {
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }
}

impl Listener for TcpSocketListener {
    fn accept(&mut self) -> Result<RemoteChannel> {
        let (socket, _) = self.listener.accept()?;

        Ok(RemoteChannel::new(TcpTransport::new(socket)))
    }
}

pub fn bind_listener(transport: TransportType) -> Result<Box<dyn Listener + Send>> {
    let listener = match transport {
        TransportType::Unix(path) => {
            Box::new(UnixSocketListener::new(UnixListener::bind(path)?)) as Box<dyn Listener + Send>
        }
        TransportType::Tcp(addr) => {
            Box::new(TcpSocketListener::new(TcpListener::bind(addr)?)) as Box<dyn Listener + Send>
        }
    };

    Ok(listener)
}
