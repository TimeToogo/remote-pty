use std::{
    env, fs,
    io::{self, Read, Result, Write},
    os::unix::net::UnixListener,
    thread,
};

use remote_pty_common::{
    channel::{transport::unix_socket::UnixSocketTransport, Channel, RemoteChannel},
    log::debug,
    proto::{
        master::{PtyMasterCall, PtyMasterResponse, WriteStdinCall},
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcError},
    },
};
use remote_pty_master::{context::Context, handler::RemotePtyServer};

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
        .unwrap_or_else(|_| panic!("could not bind pty unix socket: {}", pty_sock))
        .accept()
        .unwrap()
        .0;
    let mut chan = RemoteChannel::new(UnixSocketTransport::new(pty_sock));

    let ctx = Context::from_pair(libc::STDIN_FILENO, libc::STDIN_FILENO);

    // init pg
    chan.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PTY, |req| {
        let req = match req {
            PtySlaveCall {
                fd: _,
                typ: PtySlaveCallType::RegisterProcess(req),
            } => req,
            req @ _ => {
                debug(format!("unexpected req: {:?}", req));
                return PtySlaveResponse::Error(TcError::EIO);
            }
        };

        let mut state = ctx.state.lock().unwrap();
        state.pgrp = req.pgrp as _;
        debug("pgrp init");

        PtySlaveResponse::Success(0)
    })
    .unwrap();

    let stdin = {
        let mut chan = chan.clone();
        thread::spawn(move || -> Result<()> {
            let mut buf = [0u8; 1024];

            loop {
                let n = io::stdin().read(&mut buf)?;
                if n == 0 {
                    break;
                }

                let res = chan
                    .send::<PtyMasterCall, PtyMasterResponse>(
                        Channel::STDIN,
                        PtyMasterCall::WriteStdin(WriteStdinCall {
                            data: buf[..n].to_vec(),
                        }),
                    )
                    .unwrap();

                match res {
                    PtyMasterResponse::WriteSuccess => continue,
                    _ => panic!("unexpected response"),
                }
            }

            panic!("stdin sock eof");
        })
    };

    let stdout = {
        let mut chan = chan.clone();
        thread::spawn(move || -> Result<()> {
            loop {
                chan.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::STDOUT, |req| {
                    let req = match req {
                        PtySlaveCall {
                            fd: _,
                            typ: PtySlaveCallType::WriteStdout(req),
                        } => req,
                        _ => panic!("unexpected request"),
                    };

                    io::stdout().write_all(req.data.as_slice()).unwrap();
                    io::stdout().flush().unwrap();

                    PtySlaveResponse::Success(0)
                })
                .unwrap();
            }
        })
    };

    let pty = {
        let mut chan = chan.clone();
        thread::spawn(move || -> Result<()> {
            loop {
                chan.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PTY, |req| {
                    RemotePtyServer::handle(&ctx, req)
                })
                .unwrap();
            }
        })
    };

    let _ = stdin.join().unwrap();
    let _ = stdout.join().unwrap();
    let _ = pty.join().unwrap();
}
