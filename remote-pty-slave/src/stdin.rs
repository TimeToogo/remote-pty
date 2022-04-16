use std::{fs::File, io::Write, os::unix::prelude::FromRawFd, thread};

use remote_pty_common::{
    channel::Channel,
    log::debug,
    proto::master::{IoError, PtyMasterCall, PtyMasterResponse},
};

use crate::{channel::get_remote_channel, conf::get_conf};

// initialisation function that executes on process startup
// this replaces the stdin fd with a fd which is driven by the remote master
#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
#[no_mangle]
pub static REMOTE_PTY_INIT_STDIN: extern "C" fn() = {
    #[cfg_attr(all(target_os = "linux", not(test)), link_section = ".text.startup")]
    #[no_mangle]
    pub extern "C" fn remote_pty_init_stdin() {
        debug("redirecting stdin");

        let conf = match get_conf() {
            Ok(conf) => conf,
            Err(err) => {
                debug(format!("failed to init config: {}", err));
                return;
            }
        };

        let mut remote_channel = match get_remote_channel(&conf) {
            Ok(chan) => chan,
            Err(err) => {
                debug(format!("failed to get remote channel: {}", err));
                return;
            }
        };

        // override existing stdin fd with a pipe and keep the write end
        let mut stdin = unsafe {
            let mut fds = [0 as libc::c_int; 2];

            if libc::pipe(&mut fds as *mut _) != 0 {
                debug("failed to create pipe");
                return;
            }

            let (read_fd, write_fd) = (fds[0], fds[1]);
            if libc::dup2(read_fd, conf.stdin_fd) == -1 {
                debug("failed to dup pipe to stdin");
                return;
            }

            File::from_raw_fd(write_fd)
        };

        // TODO: get inode of pipes and store in config
        // use inodes to check if functions should be intercepted
        // instead of fd's

        // stream remote master data to stdin
        thread::spawn(move || loop {
            remote_channel
                .receive::<PtyMasterCall, PtyMasterResponse, _>(Channel::STDIN, |req| {
                    let write = match req {
                        PtyMasterCall::WriteStdin(write) => write,
                        _ => return PtyMasterResponse::Error(IoError::EIO),
                    };

                    if let Err(err) = stdin.write_all(write.data.as_slice()) {
                        debug(format!("failed to write to stdin: {}", err));
                        return PtyMasterResponse::Error(IoError::EIO);
                    }

                    if let Err(err) = stdin.flush() {
                        debug(format!("failed to write to flush stdin: {}", err));
                        return PtyMasterResponse::Error(IoError::EIO);
                    }

                    PtyMasterResponse::WriteSuccess
                })
                .unwrap();
        });

        debug("init stdin");
    }
    remote_pty_init_stdin
};
