use std::{env, fs, os::unix::net::UnixListener};

use remote_pty_master::{
    context::Context,
    server::{listener::UnixSocketListener, Server},
};

// runs the master side of the remote pty
// this is designed to be invoked by a shell and controlling terminal
// and it will use the stdin pty as the remote pty controlled by the
// remote slave
//
// run master
// RPTY_DEBUG=1 cargo run --target x86_64-unknown-linux-musl -- /tmp/pty.sock
// run slave
// RPTY_DEBUG=1 LD_PRELOAD=/tmp/x86_64-unknown-linux-gnu/release/libremote_pty_slave.linked.so RPTY_SOCK_PATH=/tmp/pty.sock RPTY_STDIN=0 RPTY_STDOUT=1,2 RPTY_EXTRA=255 bash
fn main() {
    if unsafe { libc::isatty(libc::STDIN_FILENO) } != 1 {
        panic!("stdin is not a tty");
    }

    let mut args = env::args();
    let _ = args.next();
    let pty_sock = args.next().expect("expected pty sock path");

    let _ = fs::remove_file(&pty_sock);
    let pty_sock = UnixListener::bind(&pty_sock)
        .unwrap_or_else(|_| panic!("could not bind pty unix socket: {}", pty_sock));

    let ctx = Context::from_pair(libc::STDIN_FILENO, libc::STDIN_FILENO);

    let _ = Server::new(ctx, Box::new(UnixSocketListener::new(pty_sock)))
        .start()
        .join();
}
