use remote_pty_common::{
    channel::Channel,
    log::debug,
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, RegisterProcessCall},
        Fd,
    },
};

use crate::{channel::get_remote_channel, conf::get_conf};

// initialisation function that executes on process startup
// here we register the process with the remote master
#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
#[no_mangle]
pub static REMOTE_PTY_INIT_PGRP: extern "C" fn() = {
    #[cfg_attr(all(target_os = "linux", not(test)), link_section = ".text.startup")]
    #[no_mangle]
    pub extern "C" fn remote_pty_init_pgrp() {
        debug("pgrp init");

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

        let (pid, pgrp) = unsafe { (libc::getpid(), libc::getpgrp()) };

        let res = remote_channel.send::<PtySlaveCall, PtySlaveResponse>(
            Channel::PTY,
            PtySlaveCall {
                fd: Fd(0), // unused
                typ: PtySlaveCallType::RegisterProcess(RegisterProcessCall {
                    pid: pid as _,
                    pgrp: pgrp as _,
                }),
            },
        );

        match res {
            Ok(PtySlaveResponse::Success(_)) => {}
            Ok(res) => {
                debug(format!(
                    "failed to register pty process: unexpected response {:?}",
                    res
                ));
                return;
            }
            Err(err) => {
                debug(format!("failed to register pty process: {}", err));
                return;
            }
        };

        debug("pgrp sent");
    }
    remote_pty_init_pgrp
};
