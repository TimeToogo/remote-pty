use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveResponse, TcSendBreakCall},
    Fd,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcsendbreak.html
#[no_mangle]
pub extern "C" fn tcsendbreak(fd: libc::c_int, duration: libc::c_int) -> libc::c_int {
    handle_intercept(
        "tcsendbreak",
        fd,
        |chan| tcsendbreak_chan(chan, fd, duration),
        || unsafe { libc::tcsendbreak(fd, duration) },
    )
}

fn tcsendbreak_chan(chan: Arc<dyn RemoteChannel>, fd: libc::c_int, duration: libc::c_int) -> libc::c_int {
    // send tcsendbreak request to remote
    let req = PtySlaveCall::SendBreak(TcSendBreakCall {
        fd: Fd(fd),
        duration: duration as _
    });

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcsendbreak", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcsendbreak", err),
        _ => generic_error("tcsendbreak", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcSendBreakCall},
        Fd,
    };

    use crate::channel::mock::MockChannel;

    use super::tcsendbreak_chan;

    #[test]
    fn test_tcsendbreak() {
        let expected_req = PtySlaveCall::SendBreak(TcSendBreakCall {
            fd: Fd(1),
            duration: 10
        });
        let mock_res = PtySlaveResponse::Success(0);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = tcsendbreak_chan(Arc::new(chan), 1, 10);

        assert_eq!(res, 0);
    }
}
