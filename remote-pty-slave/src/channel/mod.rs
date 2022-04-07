pub mod unix_socket;
pub mod mock;

use std::{
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;

use remote_pty_common::proto::slave::{PtySlaveCall, PtySlaveResponse};

use self::unix_socket::{init_socket_channel, UnixSocketChannel};

// an RPC channel for sending the pty calls to the
// slave side on the remote
pub trait RemoteChannel {
    fn send(&self, call: PtySlaveCall) -> Result<PtySlaveResponse, &'static str>;
}

type ChannelType = UnixSocketChannel;

lazy_static! {
    static ref GLOBAL_CHANNEL: Mutex<Option<Arc<ChannelType>>> = Mutex::new(Option::None);
}

pub fn get_remote_channel() -> Result<Arc<ChannelType>, &'static str> {
    let mut sock = GLOBAL_CHANNEL
        .lock()
        .map_err(|_| "failed to lock channel mutex")?;

    if sock.is_none() {
        let socket = init_socket_channel()?;
        let _ = sock.insert(Arc::new(socket));
    }

    Ok(Arc::clone(sock.as_ref().unwrap()))
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixListener, env};

    use crate::channel::{GLOBAL_CHANNEL, get_remote_channel};

    #[test]
    fn test_get_remote_channel() {
        // should be none until init from first call to get_remote_channel
        assert!(GLOBAL_CHANNEL.lock().unwrap().is_none());
        
        // create temp sock
        let sock_path = "/tmp/remote-pty.sock";
        let _ = std::fs::remove_file(sock_path);
        let _sock = UnixListener::bind(sock_path).unwrap();
        env::set_var("REMOTE_PTY_SOCK_PATH", sock_path);
        
        let _chan = get_remote_channel().unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());

        let _chan = get_remote_channel().unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());

        env::remove_var("REMOTE_PTY_SOCK_PATH");
    }
}