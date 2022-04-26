use std::{
    fs::File,
    io::Read,
    os::unix::prelude::FromRawFd,
    ptr,
    sync::mpsc::channel,
    thread::{self, JoinHandle},
    time::Duration,
};

use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, WriteStdoutCall},
        Fd,
    },
};

use crate::{
    conf::{get_conf, Conf, State},
    fd::{get_inode_from_fd, get_open_fds_by_inode},
    init::is_proc_forked,
    signal::block_signals_on_thread,
};

#[cfg(target_os = "linux")]
#[link(name = "c")]
extern "C" {
    #[link_name = "stdout"]
    static mut LIBC_STDOUT: *mut libc::FILE;
    #[link_name = "stderr"]
    static mut LIBC_STDERR: *mut libc::FILE;
}

#[cfg(target_os = "linux")]
static mut STDOUT_STREAM_THREAD: Option<JoinHandle<()>> = Option::None;

// this replaces the stdout fd's with a fd which is streamed to the remote master
pub(crate) fn init_stdout(conf: &Conf, mut chan: RemoteChannel, pre_fork_state: Option<&State>) {
    debug("redirecting stdout");

    let stdout_fds = pre_fork_state
        .and_then(|s| s.stdout_inode)
        .and_then(|inode| get_open_fds_by_inode(inode).ok())
        .unwrap_or_else(|| conf.stdout_fds.clone());

    // override existing stdout fd's with a pipe and keep the read end
    let (mut stdout, inode) = unsafe {
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

        debug(format!("duping stdout to {:?} fds", stdout_fds));
        for stdout_fd in &stdout_fds {
            if libc::dup2(write_fd, *stdout_fd as _) == -1 {
                debug("failed to dup pipe to stdout");
                return;
            }
        }

        if !stdout_fds.contains(&write_fd) {
            libc::close(write_fd);
        }

        // disable output buffering
        #[cfg(target_os = "linux")]
        {
            use crate::fd::disable_input_buffering;

            let _ = disable_input_buffering(LIBC_STDOUT);
            let _ = disable_input_buffering(LIBC_STDERR);
        }

        let inode = match get_inode_from_fd(read_fd) {
            Ok(inode) => inode,
            Err(_) => return,
        };

        (File::from_raw_fd(read_fd), inode)
    };

    // capture inode of stdout pipe
    conf.update_state(|state| {
        let _ = state.stdout_inode.insert(inode);
    });

    // stream remote master data to stdin
    let stream_thread = thread::spawn(move || {
        let _ = block_signals_on_thread();

        let mut buff = [0u8; 4096];

        loop {
            let n = match stdout.read(&mut buff) {
                Ok(0) => {
                    debug("eof from stdout pipe");
                    return;
                }
                Ok(n) => n,
                Err(err) => {
                    debug(format!("failed to read from stdout: {}", err));
                    return;
                }
            };

            let res = chan
                .send::<PtySlaveCall, PtySlaveResponse>(
                    Channel::STDOUT,
                    PtySlaveCall {
                        fd: Fd(0), // not used, todo: refactor data structure
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

    // this is here to prevent the stdout thread being terminated
    // before it has a change to send the stdout buffer to the remote.
    // this occurs when there is still buffered output after the main
    // function returns killing the thread before it can read the output.
    if !is_proc_forked() {
        unsafe {
            let _ = STDOUT_STREAM_THREAD.insert(stream_thread);
            let res = libc::atexit(wait_for_output);

            debug(if res == 0 {
                "registered atexit handler"
            } else {
                "failed to register atexit handler"
            });
        }
    }

    debug("init stdout");
}

extern "C" fn wait_for_output() {
    debug("atexit: stdout");

    let conf = match get_conf() {
        Ok(conf) => conf,
        Err(err) => {
            debug(format!("failed to get conf: {}", err));
            return;
        }
    };

    if !conf.is_main_thread() {
        return;
    }

    unsafe {
        libc::fflush(ptr::null_mut());
    }

    if let Ok(state) = conf.state.lock() {
        if let Some(fds) = state
            .stdout_inode
            .and_then(|i| get_open_fds_by_inode(i).ok())
        {
            for fd in &fds {
                unsafe {
                    libc::close(*fd);
                }
            }

            debug(format!("closed {} fds pointing to stdout", fds.len()));
        }
    }

    let thread = match unsafe { STDOUT_STREAM_THREAD.take() } {
        Some(t) => t,
        None => {
            debug("failed to get stdout thread");
            return;
        }
    };

    // it's very possible there are still open fd's to the write end
    // of the stdout pipe at this point.
    // if this is the case, the stdout thread join will run indefinitely.
    // we construct a channel so we can signal when it completes
    // but only give it a grace period of 3 seconds to do so.
    // why 3 seconds? good question
    let (sender, receiver) = channel();
    thread::spawn(move || {
        let _ = block_signals_on_thread();
        let _ = thread.join();
        let _ = sender.send(1);
    });
    let res = receiver.recv_timeout(Duration::from_secs(3));

    if let Err(err) = res {
        debug(format!("could not join stdout: {:?}", err));
    }
}
