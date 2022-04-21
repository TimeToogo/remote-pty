use std::{
    fs::File,
    io::Read,
    os::unix::prelude::FromRawFd,
    sync::mpsc::channel,
    thread::{self, JoinHandle},
    time::Duration,
};

use remote_pty_common::{
    channel::Channel,
    log::debug,
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, WriteStdoutCall},
        Fd,
    },
};

use crate::{channel::get_remote_channel, conf::get_conf, fd::get_inode_from_fd};

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

            for stdout_fd in &conf.stdout_fds {
                if libc::dup2(write_fd, *stdout_fd as _) == -1 {
                    debug("failed to dup pipe to stdout");
                    return;
                }
            }

            if !conf.stdout_fds.contains(&write_fd) {
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
        let _stream_thread = thread::spawn(move || {
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

                let res = remote_channel
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
        #[cfg(target_os = "linux")]
        unsafe {
            let _ = STDOUT_STREAM_THREAD.insert(_stream_thread);

            extern "C" fn wait_for_output() {
                let conf = match get_conf() {
                    Ok(conf) => conf,
                    Err(err) => {
                        debug(format!("failed to get conf: {}", err));
                        return;
                    }
                };

                for stdout_fd in &conf.stdout_fds {
                    unsafe {
                        libc::close(*stdout_fd);
                    }
                }

                if !conf.is_main_thread() {
                    return;
                }

                let thread = match unsafe { STDOUT_STREAM_THREAD.take() } {
                    Some(t) => t,
                    None => {
                        debug("failed to get stdout thread");
                        return;
                    }
                };

                let (sender, receiver) = channel();
                thread::spawn(move || {
                    let _ = thread.join();
                    let _ = sender.send(1);
                });
                let res = receiver.recv_timeout(Duration::from_secs(3));

                match res {
                    Ok(_) => {}
                    Err(err) => debug(format!("could not join stdout: {:?}", err)),
                }
            }

            libc::atexit(wait_for_output);
        }

        debug("init stdout");
    }
    remote_pty_init_stdout
};
