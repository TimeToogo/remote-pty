use std::sync::Arc;

use errno::set_errno;
use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveResponse, PtySlaveCallType},
    Fd,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/isatty.html
#[no_mangle]
pub extern "C" fn intercept_isatty(fd: libc::c_int) -> libc::c_int {
    handle_intercept(
        "isatty",
        fd,
        |chan| isatty_chan(chan, fd),
        || unsafe { libc::isatty(fd) }
    )
}

pub(crate) fn isatty_chan(chan: Arc<dyn RemoteChannel>, fd: libc::c_int) -> libc::c_int {
    // send isatty request to remote
    let req = PtySlaveCall { fd: Fd(fd), typ: PtySlaveCallType::IsATty };

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("isatty", msg),
    };

    let ret = match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("isatty", err),
        _ => generic_error("isatty", "unexpected response"),
    };

    if ret == 0 {
        set_errno(errno::Errno(libc::ENOTTY));
    }

    ret
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, PtySlaveCallType},
        Fd,
    };

    use crate::channel::mock::MockChannel;

    use super::isatty_chan;

    #[test]
    fn test_isatty() {
        let expected_req = PtySlaveCall { fd: Fd(1), typ: PtySlaveCallType::IsATty };
        let mock_res = PtySlaveResponse::Success(1);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = isatty_chan(Arc::new(chan), 1);

        assert_eq!(res, 1);
    }

    #[test]
    fn test_isatty_false() {
        let expected_req = PtySlaveCall { fd: Fd(1), typ: PtySlaveCallType::IsATty };
        let mock_res = PtySlaveResponse::Success(0);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = isatty_chan(Arc::new(chan), 1);

        assert_eq!(res, 0);
        assert_eq!(errno::errno().0, libc::ENOTTY);
    }
}
