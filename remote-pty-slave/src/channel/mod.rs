pub mod mock;
pub mod unix_socket;

use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;

use remote_pty_common::proto::slave::{PtySlaveCall, PtySlaveResponse};

use crate::conf::Conf;

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

pub fn get_remote_channel(conf: &Conf) -> Result<Arc<dyn RemoteChannel>, &'static str> {
    let mut sock = GLOBAL_CHANNEL
        .lock()
        .map_err(|_| "failed to lock channel mutex")?;

    if sock.is_none() {
        let socket = init_socket_channel(conf)?;
        let _ = sock.insert(Arc::new(socket));
    }

    let sock = Arc::clone(sock.as_ref().unwrap());
    Ok(Arc::clone(&(sock as Arc<dyn RemoteChannel>)))
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use crate::{
        channel::{get_remote_channel, GLOBAL_CHANNEL},
        conf::Conf,
    };

    #[test]
    fn test_get_remote_channel() {
        // should be none until init from first call to get_remote_channel
        assert!(GLOBAL_CHANNEL.lock().unwrap().is_none());

        // create temp sock
        let sock_path = "/tmp/remote-pty.sock";
        let _ = std::fs::remove_file(sock_path);
        let _sock = UnixListener::bind(sock_path).unwrap();
        let conf = Conf {
            sock_path: sock_path.to_string(),
            fds: vec![],
        };

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());
    }
}
