use std::{
    io,
    net::TcpStream,
    os::unix::{net::UnixStream, prelude::AsRawFd, prelude::FromRawFd},
    sync::Mutex,
};

use errno::set_errno;
use lazy_static::lazy_static;

use remote_pty_common::{
    channel::{
        transport::{conf::TransportType, rw::ReadWriteTransport},
        RemoteChannel,
    },
    log::debug,
};

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
        let new_chan = init_channel(conf)?;
        let _ = chan.insert(new_chan);
    }

    Ok(chan.as_ref().unwrap().clone())
}

trait FdConvertable: AsRawFd + FromRawFd + io::Read + io::Write + Send {}
impl<T: AsRawFd + FromRawFd + io::Read + io::Write + Send> FdConvertable for T {}

fn init_channel(conf: &Conf) -> Result<RemoteChannel, String> {
    let orig_errno = errno::errno();
    let chan = match &conf.transport {
        TransportType::Unix(sock_path) => {
            process_transport(conf, UnixStream::connect(sock_path), |i| i.try_clone())
        }
        TransportType::Tcp(sock_addr) => {
            process_transport(conf, TcpStream::connect(sock_addr), |i| i.try_clone())
        }
    };
    set_errno(orig_errno);

    chan
}

fn process_transport<T: FdConvertable + 'static, C>(
    conf: &Conf,
    transport: Result<T, io::Error>,
    try_clone: C,
) -> Result<RemoteChannel, String>
where
    C: Fn(&T) -> Result<T, io::Error>,
{
    let transport = match transport {
        Ok(t) => t,
        Err(e) => {
            return Err(format!(
                "failed to connect to transport {:?}: {}",
                conf.transport, e
            ))
        }
    };

    let transport_read = ensure_not_stdio_fd(conf, transport)?;
    let transport_write = ensure_not_stdio_fd(
        conf,
        try_clone(&transport_read).map_err(|_| "failed to clone socket")?,
    )?;
    let transport = ReadWriteTransport::new(transport_read, transport_write);

    Ok(RemoteChannel::new(transport))
}

fn ensure_not_stdio_fd<T: FdConvertable>(conf: &Conf, transport: T) -> Result<T, String> {
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

pub(crate) fn close_remote_channel() -> Result<(), String> {
    debug("closing remote channel");

    let mut chan = GLOBAL_CHANNEL
        .lock()
        .map_err(|_| "failed to lock channel mutex")?;

    let _ = chan.take();

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{os::unix::net::UnixListener, sync::Mutex};

    use remote_pty_common::channel::transport::conf::TransportType;

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
            transport: TransportType::Unix(sock_path.to_string()),
            stdin_fd: 0,
            stdout_fds: vec![],
            state: Mutex::new(State::new()),
        };

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());

        let _chan = get_remote_channel(&conf).unwrap();
        assert!(GLOBAL_CHANNEL.lock().is_ok());
    }
}
