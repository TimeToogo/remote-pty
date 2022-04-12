use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, SetProcGroupCall},
    Fd,
};

use crate::{
    channel::RemoteChannel,
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
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    pgrp: libc::pid_t,
) -> libc::c_int {
    // send tcsetpgrp request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::SetProgGroup(SetProcGroupCall { pid: pgrp as _ }),
    };

    let res = match chan.send(req) {
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
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, SetProcGroupCall},
        Fd,
    };

    use crate::{channel::mock::MockChannel, intercept::tcsetpgrp_chan};

    #[test]
    fn test_tcsetpgrp() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::SetProgGroup(SetProcGroupCall { pid: 123 }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = tcsetpgrp_chan(Arc::new(chan), 1, 123);

        assert_eq!(res, 0);
    }
}
