use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcFlushCall, TcFlushQueueSelector},
    Fd,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcflush.html
#[no_mangle]
pub extern "C" fn intercept_tcflush(fd: libc::c_int, queue_selector: libc::c_int) -> libc::c_int {
    handle_intercept(
        "tcflush",
        fd,
        |chan| tcflush_chan(chan, fd, queue_selector),
        || unsafe { libc::tcflush(fd, queue_selector) },
    )
}

pub(crate) fn tcflush_chan(
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    queue_selector: libc::c_int,
) -> libc::c_int {
    let queue_selector = match queue_selector {
        libc::TCIFLUSH => TcFlushQueueSelector::TCIFLUSH,
        libc::TCOFLUSH => TcFlushQueueSelector::TCOFLUSH,
        libc::TCIOFLUSH => TcFlushQueueSelector::TCIOFLUSH,
        _ => {
            return generic_error(
                "tcflush",
                format!("invalid queue selector {}", queue_selector),
            )
        }
    };

    // send tcflush request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Flush(TcFlushCall { queue_selector }),
    };

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcflush", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcflush", err),
        _ => generic_error("tcflush", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcFlushCall, TcFlushQueueSelector},
        Fd,
    };

    use crate::channel::mock::MockChannel;

    use super::tcflush_chan;

    #[test]
    fn test_tcflush() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: remote_pty_common::proto::slave::PtySlaveCallType::Flush(TcFlushCall {
                queue_selector: TcFlushQueueSelector::TCIOFLUSH,
            }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = tcflush_chan(Arc::new(chan), 1, libc::TCIOFLUSH);

        assert_eq!(res, 0);
    }
}
