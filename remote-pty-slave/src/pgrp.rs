use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, RegisterProcessCall},
        Fd,
    },
};

// here we register the process with the remote master
pub(crate) fn register_process(chan: &mut RemoteChannel) -> Result<(), String> {
    debug("pgrp init");

    let (pid, pgrp) = unsafe { (libc::getpid(), libc::getpgrp()) };

    let res = chan.send::<PtySlaveCall, PtySlaveResponse>(
        Channel::PGRP,
        PtySlaveCall {
            fd: Fd(0), // unused
            typ: PtySlaveCallType::RegisterProcess(RegisterProcessCall {
                pid: pid as _,
                pgrp: pgrp as _,
            }),
        },
    );

    let res = match res {
        Ok(PtySlaveResponse::Success(_)) => {
            debug("pgrp sent");
            Ok(())
        }
        Ok(res) => {
            Err(format!(
                "failed to register pty process: unexpected response {:?}",
                res
            ))
        }
        Err(err) => {
            Err(format!("failed to register pty process: {}", err))
            
        }
    };

    if let Err(msg) = res.as_ref() {
        debug(msg);
    }

    res
}
