use std::{ops::DerefMut, os::unix::net::UnixStream, result::Result, sync::Mutex};

use remote_pty_common::proto::slave::{PtySlaveCall, PtySlaveResponse};

use crate::conf::Conf;

use super::RemoteChannel;

#[derive(Debug)]
pub struct UnixSocketChannel {
    socket: Mutex<UnixStream>,
}

impl RemoteChannel for UnixSocketChannel {
    fn send(&self, call: PtySlaveCall) -> Result<PtySlaveResponse, &'static str> {
        let mut sock = self
            .socket
            .lock()
            .map_err(|_| "failed to lock sock mutex")?;

        let enc_conf = bincode::config::standard();
        bincode::encode_into_std_write(call, sock.deref_mut(), enc_conf)
            .map_err(|_| "failed to write pty call to socket")?;

        let response = bincode::decode_from_std_read(sock.deref_mut(), enc_conf)
            .map_err(|_| "failed to read pty response from sock")?;

        Ok(response)
    }
}

pub fn init_socket_channel(conf: &Conf) -> Result<UnixSocketChannel, &'static str> {
    let sock =
        UnixStream::connect(&conf.sock_path).map_err(|_| "failed to connect to unix socket")?;

    Ok(UnixSocketChannel {
        socket: Mutex::new(sock),
    })
}

#[cfg(test)]
mod tests {
    use std::{io::Write, os::unix::net::UnixListener, thread};

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
        Fd,
    };

    use crate::{
        channel::{unix_socket::init_socket_channel, RemoteChannel},
        conf::Conf,
    };

    #[test]
    fn test_init_invalid_path() {
        let res = init_socket_channel(&Conf {
            sock_path: "/this/is/not/valid".to_string(),
            fds: vec![],
        });

        assert!(res.is_err());
    }

    #[test]
    fn test_init_valid_path() {
        let sock_path = "/tmp/remote-pty-test-1.sock";
        let _ = std::fs::remove_file(sock_path);
        let _temp_sock = UnixListener::bind(sock_path).unwrap();
        let res = init_socket_channel(&Conf {
            sock_path: sock_path.to_string(),
            fds: vec![],
        });

        assert!(res.is_ok());
    }

    #[test]
    fn test_send_receive_msg() {
        let sock_path = "/tmp/remote-pty-test-2.sock";
        let _ = std::fs::remove_file(sock_path);

        let temp_sock = UnixListener::bind(sock_path).unwrap();
        let chan = init_socket_channel(&Conf {
            sock_path: sock_path.to_string(),
            fds: vec![],
        })
        .unwrap();

        let req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetAttr,
        };
        let res = PtySlaveResponse::Success(0);

        // accept connection and send reply in another thread
        // as sending is blocking
        let (req_copy, res_copy) = (req.clone(), res.clone());
        let reply_thread = thread::spawn(move || {
            let (mut sock, _) = temp_sock.accept().unwrap();
            let enc_conf = bincode::config::standard();

            let recv_req: PtySlaveCall =
                bincode::decode_from_std_read(&mut sock, enc_conf).unwrap();
            assert_eq!(recv_req, req_copy);

            let buf = bincode::encode_to_vec(res_copy, enc_conf).unwrap();
            sock.write_all(buf.as_slice()).unwrap();
            sock.flush().unwrap();
        });

        // receive reply on main thread
        let recv_res = chan.send(req.clone()).unwrap();

        assert_eq!(recv_res, res);
        reply_thread.join().expect("failed to join reply thread");
    }
}
