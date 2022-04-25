use std::{io, mem::MaybeUninit, ptr, thread};

use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::master::{IoError, PtyMasterCall, PtyMasterResponse, PtyMasterSignal},
};

// we forward signals from the remote master to the local process
pub(crate) fn init_signal_handler(mut chan: RemoteChannel) {
    debug("signal handler");

    thread::spawn(move || {
        let _ = block_signals_on_thread();
        loop {
            chan
                .receive::<PtyMasterCall, PtyMasterResponse, _>(Channel::SIGNAL, |req| {
                    let req = match req {
                        PtyMasterCall::Signal(sig) => sig,
                        _ => {
                            debug(format!("unexpected request: {:?}", req));
                            return PtyMasterResponse::Error(IoError::EIO);
                        }
                    };

                    debug(format!("received signal from master: {:?}", req));

                    let signal = match req.signal {
                        PtyMasterSignal::SIGWINCH => libc::SIGWINCH,
                        PtyMasterSignal::SIGINT => libc::SIGINT,
                        PtyMasterSignal::SIGTERM => libc::SIGTERM,
                        PtyMasterSignal::SIGCONT => libc::SIGCONT,
                        PtyMasterSignal::SIGTTOU => libc::SIGTTOU,
                        PtyMasterSignal::SIGTTIN => libc::SIGTTIN,
                    };

                    let ret = unsafe { libc::kill(req.pgrp as _, signal) };

                    if ret == -1 {
                        debug(format!(
                            "failed to send signal to local process: {}",
                            errno::errno()
                        ));
                        return PtyMasterResponse::Error(IoError::EIO);
                    }

                    PtyMasterResponse::Success(0)
                })
                .unwrap();
        }
    });

    debug("init signal handler");
}

// block all signals on the calling thread
pub(crate) fn block_signals_on_thread() -> io::Result<()> {
    unsafe {
        let mut sigset = MaybeUninit::<libc::sigset_t>::zeroed().assume_init();

        if libc::sigfillset(&mut sigset as *mut _) == -1 {
            debug("failed to fill sigset");
            return Err(io::Error::last_os_error());
        }

        if libc::pthread_sigmask(libc::SIG_SETMASK, &sigset as *const _, ptr::null_mut()) == -1 {
            debug("failed to set thread sigmask");
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}
