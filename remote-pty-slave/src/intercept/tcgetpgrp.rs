use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
        Fd,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// @see https://man7.org/linux/man-pages/man3/tcgetpgrp.3.html
#[no_mangle]
pub extern "C" fn intercept_tcgetpgrp(fd: libc::c_int) -> libc::pid_t {
    handle_intercept(
        format!("tcgetpgrp({})", fd),
        fd,
        |chan| tcgetpgrp_chan(chan, fd),
        || unsafe { libc::tcgetpgrp(fd) },
    )
}

pub(crate) fn tcgetpgrp_chan(mut chan: RemoteChannel, fd: libc::pid_t) -> libc::pid_t {
    // send tcgetpgrp request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::GetProcGroup,
    };

    let res = match chan.send(Channel::PTY, req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcgetpgrp", msg),
    };

    let res = match res {
        PtySlaveResponse::GetProcGroup(res) => res,
        PtySlaveResponse::Error(err) => return tc_error("tcgetpgrp", err),
        _ => return generic_error("tcgetpgrp", "unexpected response"),
    };

    res.pid as _
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{ProcGroupResponse, PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
            Fd,
        },
    };

    use super::tcgetpgrp_chan;

    #[test]
    fn test_tcgetpgrp() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetProcGroup,
        };
        let mock_res = PtySlaveResponse::GetProcGroup(ProcGroupResponse { pid: 1234 });
        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = tcgetpgrp_chan(mock.chan.clone(), 1);

        assert_eq!(res, 1234);
    }
}
