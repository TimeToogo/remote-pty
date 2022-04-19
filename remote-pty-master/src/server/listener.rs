use std::{io::Result, os::unix::net::UnixListener};

use remote_pty_common::channel::{RemoteChannel, transport::unix_socket::UnixSocketTransport};

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

