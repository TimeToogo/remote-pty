use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, SetProcessGroupCall},
        Fd,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// @see https://www.man7.org/linux/man-pages/man2/setpgid.2.html
#[no_mangle]
pub extern "C" fn setpgid(pid: libc::pid_t, pgrp: libc::pid_t) -> libc::c_int {
    handle_intercept(
        format!("setpgid({}, {})", pid, pgrp),
        0, // unused
        |chan| setpgid_chan(chan, pid, pgrp),
        || unsafe { __libc__setpgid(pid, pgrp) },
    )
}

#[no_mangle]
pub extern "C" fn setpgrp(pid: libc::pid_t, pgrp: libc::pid_t) -> libc::c_int {
    setpgid(pid, pgrp)
}

#[cfg(all(not(test), target_env = "musl"))]
extern "C" {
    // symbol overridden during build scripts
    fn __libc__setpgid(pid: libc::pid_t, pgrp: libc::pid_t) -> libc::c_int;
}

#[cfg(any(test, target_os = "macos", target_env = "gnu"))]
#[no_mangle]
#[allow(non_snake_case)]
unsafe fn __libc__setpgid(pid: libc::pid_t, pgrp: libc::pid_t) -> libc::c_int {
    let setpgid = libc::dlsym(libc::RTLD_NEXT, "setpgid\0".as_ptr() as *const _);

    if setpgid.is_null() {
        panic!("unable to find setpgid sym");
    }

    let setpgid = std::mem::transmute::<
        _,
        unsafe extern "C" fn(pid: libc::pid_t, pgrp: libc::pid_t) -> libc::c_int,
    >(setpgid);

    setpgid(pid, pgrp)
}

pub(crate) fn setpgid_chan(
    mut chan: RemoteChannel,
    pid: libc::pid_t,
    pgrp: libc::pid_t,
) -> libc::c_int {
    // we run the local setpgid and then, only if it was successful
    // do we apply the same change on the server

    let ret = unsafe { __libc__setpgid(pid, pgrp) };

    if ret != 0 {
        return ret;
    }

    let req = PtySlaveCall {
        fd: Fd(0),
        typ: PtySlaveCallType::SetProcessGroup(SetProcessGroupCall {
            pid: pid as _,
            new_pgrp: pgrp as _,
        }),
    };

    let res = match chan.send(Channel::PTY, req) {
        Ok(res) => res,
        Err(msg) => return generic_error("setpgid", msg),
    };

    match res {
        PtySlaveResponse::Success(_) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("setpgid", err),
        _ => generic_error("setpgid", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{mock::MockChannel, Channel},
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, SetProcessGroupCall},
            Fd,
        },
    };

    use crate::intercept::setpgid_chan;

    #[test]
    fn test_setpgid() {
        let expected_req = PtySlaveCall {
            fd: Fd(0),
            typ: PtySlaveCallType::SetProcessGroup(SetProcessGroupCall {
                pid: unsafe { libc::getpid() } as _,
                new_pgrp: 0,
            }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = setpgid_chan(mock.chan.clone(), unsafe { libc::getpid() } as _, 0);

        assert_eq!(res, 0);
    }
}
