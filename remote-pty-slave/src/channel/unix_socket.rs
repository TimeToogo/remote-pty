use std::{
    env,
    os::unix::net::UnixStream,
    result::Result,
    sync::{Mutex}, ops::DerefMut,
};

use remote_pty_common::proto::slave::{PtySlaveCall, PtySlaveResponse};

use super::RemoteChannel;

#[derive(Debug)]
pub struct UnixSocketChannel {
    socket: Mutex<UnixStream>,
}

impl RemoteChannel for UnixSocketChannel {
    fn send(&self, call: PtySlaveCall) -> Result<PtySlaveResponse, &'static str> {
        let mut sock = self.socket.lock()
            .map_err(|_| "failed to lock sock mutex")?;

        let enc_conf = bincode::config::standard();
        bincode::encode_into_std_write(call, sock.deref_mut(), enc_conf)
            .map_err(|_| "failed to write pty call to socket")?;

        let response = bincode::decode_from_std_read(sock.deref_mut(), enc_conf)
            .map_err(|_| "failed to read pty response from sock")?;

        Ok(response)
    }
}

pub fn init_socket_channel() -> Result<UnixSocketChannel, &'static str> {
    let sock_path =
        env::var("REMOTE_PTY_SOCK_PATH").map_err(|_| "could not get REMOTE_PTY_SOCK_PATH")?;

    init_socket_channel_path(sock_path.as_str())
}

fn init_socket_channel_path(sock_path: &str) -> Result<UnixSocketChannel, &'static str> {
    let sock = UnixStream::connect(sock_path).map_err(|_| "failed to connect to unix socket")?;

    Ok(UnixSocketChannel {
        socket: Mutex::new(sock),
    })
}


#[cfg(test)]
mod tests {
    use std::{os::unix::{net::{UnixListener}}, thread, io::Write};

    use remote_pty_common::proto::{slave::{PtySlaveCall, TcGetAttrCall, PtySlaveResponse}, Fd};

    use crate::channel::RemoteChannel;

    use super::init_socket_channel_path;

    #[test]
    fn test_init_invalid_path() {
        let res = init_socket_channel_path("/this/is/not/valid");

        assert!(res.is_err());
    }

    #[test]
    fn test_init_valid_path() {
        let sock_path = "/tmp/remote-pty-test-1.sock";
        let _ = std::fs::remove_file(sock_path);
        let _temp_sock = UnixListener::bind(sock_path).unwrap();
        let res = init_socket_channel_path(sock_path);

        assert!(res.is_ok());
    }

    #[test]
    fn test_send_receive_msg() {
        let sock_path = "/tmp/remote-pty-test-2.sock";
        let _ = std::fs::remove_file(sock_path);

        let temp_sock = UnixListener::bind(sock_path).unwrap();
        let chan = init_socket_channel_path(sock_path).unwrap();

        let req = PtySlaveCall::GetAttr(TcGetAttrCall { fd: Fd(1) });
        let res = PtySlaveResponse::Success;

        // accept connection and send reply in another thread
        // as sending is blocking
        let (req_copy, res_copy) = (req.clone(), res.clone());
        let reply_thread = thread::spawn(move || {
            let (mut sock, _) = temp_sock.accept().unwrap();
            let enc_conf = bincode::config::standard();

            let recv_req: PtySlaveCall = bincode::decode_from_std_read(&mut sock, enc_conf).unwrap();
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