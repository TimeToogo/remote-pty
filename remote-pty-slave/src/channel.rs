use std::{os::unix::net::UnixStream, sync::Mutex};

use errno::set_errno;
use lazy_static::lazy_static;

use remote_pty_common::channel::{transport::unix_socket::UnixSocketTransport, RemoteChannel};

use crate::conf::Conf;

lazy_static! {
    static ref GLOBAL_CHANNEL: Mutex<Option<RemoteChannel>> = Mutex::new(Option::None);
}

pub fn get_remote_channel(conf: &Conf) -> Result<RemoteChannel, String> {
    let mut chan = GLOBAL_CHANNEL
        .lock()
        .map_err(|_| "failed to lock channel mutex")?;

    if chan.is_none() {
        let orig_errno = errno::errno();
        let transport = UnixStream::connect(&conf.sock_path);
        set_errno(orig_errno);

        if let Err(e) = transport {
            return Err(format!("failed to connect to unix socket: {}", e));
        }

        let transport = UnixSocketTransport::new(transport.unwrap());
        let _ = chan.insert(RemoteChannel::new(transport));
    }

    Ok(chan.as_ref().unwrap().clone())
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
            stdin_fd: 0,
            stdout_fds: vec![],
            pty_fds: vec![],
        };

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());
    }
}
