use std::{fs::File, io::Write, os::unix::prelude::FromRawFd, thread};

use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::master::{IoError, PtyMasterCall, PtyMasterResponse},
};

use crate::{
    conf::{Conf, State},
    fd::{get_inode_from_fd, get_open_fds_by_inode},
    signal::block_signals_on_thread,
};

#[cfg(target_os = "linux")]
#[link(name = "c")]
extern "C" {
    #[link_name = "stdin"]
    static mut LIBC_STDIN: *mut libc::FILE;
}

// this replaces the stdin fd with a fd which is driven by the remote master
pub(crate) fn init_stdin(conf: &Conf, mut chan: RemoteChannel, pre_fork_state: Option<&State>) {
    debug("redirecting stdin");

    let stdin_fds = pre_fork_state
        .and_then(|s| s.stdin_inode)
        .and_then(|inode| get_open_fds_by_inode(inode).ok())
        .unwrap_or_else(|| vec![conf.stdin_fd]);

    // override existing stdin fd with a pipe and keep the write end
    let (mut stdin, inode) = unsafe {
        let mut fds = [0 as libc::c_int; 2];

        #[cfg(target_os = "linux")]
        let res = libc::pipe2(&mut fds as *mut _, libc::O_CLOEXEC);
        #[cfg(not(target_os = "linux"))]
        let res = libc::pipe(&mut fds as *mut _);
        if res != 0 {
            debug("failed to create pipe");
            return;
        }

        let (read_fd, write_fd) = (fds[0], fds[1]);

        debug(format!("duping stdin to {:?} fds", stdin_fds));
        for stdin_fd in &stdin_fds {
            if libc::dup2(read_fd, *stdin_fd) == -1 {
                debug("failed to dup pipe to stdin");
                return;
            }
        }

        #[cfg(target_os = "linux")]
        {
            use crate::fd::disable_input_buffering;
            let _ = disable_input_buffering(LIBC_STDIN);
        }

        let inode = match get_inode_from_fd(write_fd) {
            Ok(inode) => inode,
            Err(_) => return,
        };

        (File::from_raw_fd(write_fd), inode)
    };

    // capture inode of stdin pipe
    conf.update_state(|state| {
        let _ = state.stdin_inode.insert(inode);
    });

    // stream remote master data to stdin
    thread::spawn(move || {
        let _ = block_signals_on_thread();

        loop {
            chan.receive::<PtyMasterCall, PtyMasterResponse, _>(Channel::STDIN, |req| {
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
        }
    });

    debug("init stdin");
}
