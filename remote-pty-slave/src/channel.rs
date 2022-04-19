use std::{
    os::unix::{net::UnixStream, prelude::AsRawFd, prelude::FromRawFd},
    sync::Mutex,
};

use errno::set_errno;
use lazy_static::lazy_static;

use remote_pty_common::channel::{transport::rw::ReadWriteTransport, RemoteChannel};

use crate::conf::Conf;

lazy_static! {
    static ref GLOBAL_CHANNEL: Mutex<Option<RemoteChannel>> = Mutex::new(Option::None);
}

pub fn get_remote_channel(conf: &Conf) -> Result<RemoteChannel, String> {
    lazy_static::initialize(&GLOBAL_CHANNEL);

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

        let transport_read = ensure_not_stdio_fd(conf, transport.unwrap())?;
        let transport_write = ensure_not_stdio_fd(
            conf,
            transport_read
                .try_clone()
                .map_err(|_| "failed to clone unix socket")?,
        )?;
        let transport = ReadWriteTransport::new(transport_read, transport_write);
        let _ = chan.insert(RemoteChannel::new(transport));
    }

    Ok(chan.as_ref().unwrap().clone())
}

fn ensure_not_stdio_fd<T: AsRawFd + FromRawFd>(conf: &Conf, transport: T) -> Result<T, String> {
    let fd = transport.as_raw_fd();
    let mut new_fd = 255;

    while conf.is_stdio_fd(fd) || is_fd_taken(new_fd) {
        new_fd -= 1;

        if new_fd == 0 {
            return Err("failed to find available fd for transport channel".to_string());
        }
    }

    let res = unsafe { libc::dup2(fd, new_fd) };

    if res == -1 {
        return Err(format!("failed to dup transport fd to new fd {}", new_fd));
    }

    let res = unsafe { libc::close(fd) };

    if res == -1 {
        return Err(format!("failed to close original transport fd {}", fd));
    }

    Ok(unsafe { T::from_raw_fd(new_fd) })
}

fn is_fd_taken(fd: libc::c_int) -> bool {
    return unsafe { libc::fcntl(fd, libc::F_GETFL) } != -1 || errno::errno().0 != libc::EBADF;
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixListener, sync::Mutex};

    use crate::{
        channel::{get_remote_channel, GLOBAL_CHANNEL},
        conf::{Conf, State},
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
            thread_id: 1,
            state: Mutex::new(State::new())
        };

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());
    }
}
