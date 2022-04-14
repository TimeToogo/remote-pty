use std::{fs::File, io::Read, os::unix::prelude::FromRawFd, thread};

use remote_pty_common::{
    channel::Channel,
    log::debug,
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, WriteStdoutCall},
        Fd,
    },
};

use crate::{channel::get_remote_channel, conf::get_conf};

// initialisation function that executes on process startup
// this replaces the stdout fd's with a fd which is streamed to the remote master
#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
#[no_mangle]
pub static REMOTE_PTY_INIT_STDOUT: extern "C" fn() = {
    #[cfg_attr(all(target_os = "linux", not(test)), link_section = ".text.startup")]
    #[no_mangle]
    pub extern "C" fn remote_pty_init_stdout() {
        debug("redirecting stdout");

        let conf = match get_conf() {
            Ok(conf) => conf,
            Err(err) => panic!("failed to init config: {}", err),
        };

        let mut remote_channel = match get_remote_channel(&conf) {
            Ok(chan) => chan,
            Err(err) => {
                debug(format!("failed to get remote channel: {}", err));
                return;
            }
        };

        // override existing stdout fd's with a pipe and keep the read end
        let mut stdout = unsafe {
            let mut fds = [0 as libc::c_int; 2];

            if libc::pipe(&mut fds as *mut _) != 0 {
                debug("failed to create pipe");
                return;
            }

            let (read_fd, write_fd) = (fds[0], fds[1]);

            for stdout_fd in &conf.stdout_fds {
                libc::close(*stdout_fd as _);
                if libc::dup2(write_fd, *stdout_fd as _) == -1 {
                    debug("failed to dup pipe to stdout");
                    return;
                }
            }

            File::from_raw_fd(read_fd)
        };

        // stream remote master data to stdin
        thread::spawn(move || {
            let mut buff = [0u8; 4096];

            loop {
                let n = match stdout.read(&mut buff) {
                    Ok(n) => n,
                    Err(err) => {
                        debug(format!("failed to read from stdout: {}", err));
                        return;
                    }
                };

                let res = remote_channel
                    .send::<PtySlaveCall, PtySlaveResponse>(
                        Channel::STDOUT,
                        PtySlaveCall {
                            fd: Fd(0), // not used: todo clean data structure
                            typ: PtySlaveCallType::WriteStdout(WriteStdoutCall {
                                data: buff[..n].to_vec(),
                            }),
                        },
                    )
                    .unwrap();

                match res {
                    PtySlaveResponse::Success(_) => continue,
                    res @ _ => {
                        debug(format!("expected response from master: {:?}", res));
                        return;
                    }
                }
            }
        });

        debug("init stdout");
    }
    remote_pty_init_stdout
};
