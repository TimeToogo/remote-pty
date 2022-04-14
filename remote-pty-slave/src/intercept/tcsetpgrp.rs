use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, SetProcGroupCall},
        Fd,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// non-standard but equivalent to ioctl(fd, TCIOSWINSZ, *pgrp)
// @see https://fossies.org/dox/musl-1.2.2/tcsetpgrp_8c_source.html
#[no_mangle]
pub extern "C" fn intercept_tcsetpgrp(fd: libc::c_int, pgrp: libc::pid_t) -> libc::c_int {
    handle_intercept(
        format!("tcsetpgrp({}, {})", fd, pgrp),
        fd,
        |chan| tcsetpgrp_chan(chan, fd, pgrp),
        || unsafe { libc::tcsetpgrp(fd, pgrp) },
    )
}

pub(crate) fn tcsetpgrp_chan(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    pgrp: libc::pid_t,
) -> libc::c_int {
    // send tcsetpgrp request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::SetProgGroup(SetProcGroupCall { pid: pgrp as _ }),
    };

    let res = match chan.send(Channel::PTY, req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcsetpgrp", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcsetpgrp", err),
        _ => generic_error("tcsetpgrp", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, SetProcGroupCall},
            Fd,
        },
    };

    use crate::intercept::tcsetpgrp_chan;

    #[test]
    fn test_tcsetpgrp() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::SetProgGroup(SetProcGroupCall { pid: 123 }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = tcsetpgrp_chan(mock.chan.clone(), 1, 123);

        assert_eq!(res, 0);
    }
}
