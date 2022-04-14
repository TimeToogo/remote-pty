use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcSendBreakCall},
        Fd,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcsendbreak.html
#[no_mangle]
pub extern "C" fn intercept_tcsendbreak(fd: libc::c_int, duration: libc::c_int) -> libc::c_int {
    handle_intercept(
        format!("tcsendbreak({}, ...)", fd),
        fd,
        |chan| tcsendbreak_chan(chan, fd, duration),
        || unsafe { libc::tcsendbreak(fd, duration) },
    )
}

pub(crate) fn tcsendbreak_chan(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    duration: libc::c_int,
) -> libc::c_int {
    // send tcsendbreak request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::SendBreak(TcSendBreakCall {
            duration: duration as _,
        }),
    };

    let res = match chan.send(Channel::PTY, req) {
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
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcSendBreakCall},
            Fd,
        },
    };

    use super::tcsendbreak_chan;

    #[test]
    fn test_tcsendbreak() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::SendBreak(TcSendBreakCall { duration: 10 }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = tcsendbreak_chan(mock.chan.clone(), 1, 10);

        assert_eq!(res, 0);
    }
}
