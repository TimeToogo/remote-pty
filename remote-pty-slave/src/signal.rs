use std::thread;

use remote_pty_common::{
    channel::Channel,
    log::debug,
    proto::master::{IoError, PtyMasterCall, PtyMasterResponse, PtyMasterSignal},
};

use crate::{channel::get_remote_channel, conf::get_conf};

// initialisation function that executes on process startup
// here we forward signals from the remote master to the local process
#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
#[no_mangle]
pub static REMOTE_PTY_INIT_SIGNAL_HANDLER: extern "C" fn() = {
    #[cfg_attr(all(target_os = "linux", not(test)), link_section = ".text.startup")]
    #[no_mangle]
    pub extern "C" fn remote_pty_init_signal_handler() {
        debug("signal handler");

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

        thread::spawn(move || loop {
            remote_channel
                .receive::<PtyMasterCall, PtyMasterResponse, _>(Channel::SIGNAL, |req| {
                    let signal = match req {
                        PtyMasterCall::Signal(sig) => sig,
                        _ => {
                            debug(format!("unexpected request: {:?}", req));
                            return PtyMasterResponse::Error(IoError::EIO);
                        }
                    };

                    debug(format!("received signal from master: {:?}", signal));

                    let signal = match signal {
                        PtyMasterSignal::SIGWINCH => libc::SIGWINCH,
                        PtyMasterSignal::SIGINT => libc::SIGINT,
                        PtyMasterSignal::SIGTERM => libc::SIGTERM,
                        PtyMasterSignal::SIGCONT => libc::SIGCONT,
                        PtyMasterSignal::SIGTTOU => libc::SIGTTOU,
                        PtyMasterSignal::SIGTTIN => libc::SIGTTIN,
                    };

                    let ret = unsafe { libc::kill(libc::getpid(), signal) };

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
        });

        debug("init signal handler");
    }
    remote_pty_init_signal_handler
};
