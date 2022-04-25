use std::sync::atomic::{AtomicU32, Ordering};

use remote_pty_common::log::debug;

use crate::{
    channel::get_remote_channel, conf::get_conf, fork::fork_handler, pgrp::register_process,
    signal::init_signal_handler, stdin::init_stdin, stdout::init_stdout,
};

// track the number of calls to the init
// so we can determine if the process is initialised through process
// startup or forking
static INIT_COUNTER: AtomicU32 = AtomicU32::new(0);

#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
#[no_mangle]
pub static REMOTE_PTY_INIT: extern "C" fn() = remote_pty_init;

// initialisation function that executes on process startup
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".text.startup")]
#[no_mangle]
pub extern "C" fn remote_pty_init() {
    debug("process init");

    INIT_COUNTER.fetch_add(1, Ordering::SeqCst);

    let conf = match get_conf() {
        Ok(conf) => conf,
        Err(err) => {
            debug(format!("failed to init config: {}", err));
            return;
        }
    };

    let mut chan = match get_remote_channel(&conf) {
        Ok(chan) => chan,
        Err(err) => {
            debug(format!("failed to get remote channel: {}", err));
            return;
        }
    };

    let res = register_process(&mut chan);

    if res.is_err() {
        debug("init failed: could not register process");
        return;
    }

    init_signal_handler(chan.clone());
    init_stdin(&conf, chan.clone());
    init_stdout(&conf, chan.clone());

    if !is_proc_forked() {
        unsafe {
            let res = libc::pthread_atfork(None, None, Some(fork_handler));

            debug(if res == 0 {
                "registered atfork handler"
            } else {
                "failed to register atfork handler"
            });
        }
    }

    debug("init complete");
}

pub(crate) fn is_proc_forked() -> bool {
    return INIT_COUNTER.load(Ordering::SeqCst) > 1;
}
