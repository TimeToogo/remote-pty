use std::{env, fs};

use remote_pty_common::channel::transport::conf::TransportType;
use remote_pty_master::{
    context::Context,
    server::{listener::bind_listener, Server},
};

// runs the master side of the remote pty
// this is designed to be invoked by a shell and controlling terminal
// and it will use the stdin pty as the remote pty controlled by the
// remote slave
//
// run master
// RPTY_DEBUG=1 cargo run --target x86_64-unknown-linux-musl -- unix:/tmp/pty.sock
// run slave
// RPTY_DEBUG=1 LD_PRELOAD=/tmp/x86_64-unknown-linux-gnu/release/libremote_pty_slave.linked.so RPTY_TRANSPORT=unix:/tmp/pty.sock bash
fn main() {
    if unsafe { libc::isatty(libc::STDIN_FILENO) } != 1 {
        panic!("stdin is not a tty");
    }

    let mut args = env::args();
    let _ = args.next();
    let transport = args
        .next()
        .expect("expected pty transport")
        .parse::<TransportType>()
        .expect("could not parse transport");

    if let TransportType::Unix(path) = &transport {
        let _ = fs::remove_file(path);
    }
    let listener =
        bind_listener(transport).unwrap_or_else(|e| panic!("could not bind listener: {}", e));

    let ctx = Context::from_pair(libc::STDIN_FILENO, libc::STDIN_FILENO);

    let _ = Server::new(ctx, listener).start().join();
}
