use std::{
    env, fs,
    io::{self, Read, Result, Write},
    os::unix::net::UnixListener,
    thread,
};

use remote_pty_master::{context::Context, handler::RemotePtyServer};

// runs the master side of the remote pty
// this is designed to be invoked by a shell and controlling terminal
// and it will use the stdin pty as the remote pty controlled by the
// remote slave
//
// run master
// RPTY_DEBUG=1 cargo run --target x86_64-unknown-linux-musl -- /tmp/pty.sock /tmp/stdin.sock /tmp/stdout.sock
// run slave
// nc -U /tmp/stdin.sock | RPTY_DEBUG=1 RPTY_SOCK_PATH=/tmp/pty.sock RPTY_FDS=0,1,2 ./bash-linux-x86_64 2>&1 | nc -U /tmp/stdout.sock
fn main() {
    if unsafe { libc::isatty(libc::STDIN_FILENO) } != 1 {
        panic!("stdin is not a tty");
    }

    let mut args = env::args();
    let _ = args.next();
    let pty_sock = args.next().expect("expected pty sock path");
    let stdin_sock = args.next().expect("expected stdin sock path");
    let stdout_sock = args.next().expect("expected stdout sock path");

    let _ = fs::remove_file(&pty_sock);
    let _ = fs::remove_file(&stdin_sock);
    let _ = fs::remove_file(&stdout_sock);

    // disable io buffering
    // libc::setvbuf(libc::stdio, buffer, mode, size)

    let ctx = Context::from_pair(libc::STDIN_FILENO, libc::STDIN_FILENO);

    let reader = thread::spawn(move || -> Result<()> {
        let mut stdin_sock = UnixListener::bind(&stdin_sock)
            .unwrap_or_else(|_| panic!("could not bind stdin unix socket: {}", stdin_sock))
            .accept()
            .unwrap()
            .0;

        let mut buf = [0u8; 1024];

        loop {
            let n = io::stdin().read(&mut buf)?;
            if n == 0 {
                break;
            }

            stdin_sock.write_all(&buf[..n])?;
            stdin_sock.flush()?;
        }

        panic!("stdin sock eof");
    });

    let writer = thread::spawn(move || -> Result<()> {
        let mut stdout_sock = UnixListener::bind(&stdout_sock)
            .unwrap_or_else(|_| panic!("could not bind stdout unix socket: {}", stdout_sock))
            .accept()
            .unwrap()
            .0;

        let mut buf = [0u8; 1024];

        loop {
            let n = stdout_sock.read(&mut buf)?;
            if n == 0 {
                break;
            }

            io::stdout().write_all(&buf[..n])?;
            io::stdout().flush()?;
        }

        panic!("stdout sock eof");
    });

    let pty_handler = thread::spawn(move || -> Result<()> {
        let mut pty_sock = UnixListener::bind(&pty_sock)
            .unwrap_or_else(|_| panic!("could not bind pty unix socket: {}", pty_sock))
            .accept()
            .unwrap()
            .0;

        let conf = bincode::config::standard();

        let enc_err = |_| io::Error::from_raw_os_error(libc::EIO);
        let dec_err = |_| io::Error::from_raw_os_error(libc::EIO);

        loop {
            let req = bincode::decode_from_std_read(&mut pty_sock, conf).map_err(enc_err)?;
            let res = RemotePtyServer::handle(&ctx, req);
            bincode::encode_into_std_write(res, &mut pty_sock, conf).map_err(dec_err)?;
        }
    });

    let _ = reader.join().unwrap();
    let _ = writer.join().unwrap();
    let _ = pty_handler.join().unwrap();
}
