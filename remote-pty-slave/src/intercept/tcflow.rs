use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcFlowAction, TcFlowCall},
        Fd,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcflow.html
#[no_mangle]
pub extern "C" fn intercept_tcflow(fd: libc::c_int, action: libc::c_int) -> libc::c_int {
    handle_intercept(
        format!("tcflow({})", fd),
        fd,
        |chan| tcflow_chan(chan, fd, action),
        || unsafe { libc::tcflow(fd, action) },
    )
}

pub(crate) fn tcflow_chan(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    action: libc::c_int,
) -> libc::c_int {
    let action = match action {
        libc::TCOON => TcFlowAction::TCOON,
        libc::TCOOFF => TcFlowAction::TCOOFF,
        libc::TCIOFF => TcFlowAction::TCIOFF,
        libc::TCION => TcFlowAction::TCION,
        _ => return generic_error("tcflow", format!("invalid action {}", action)),
    };

    // send tcflow request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Flow(TcFlowCall { action }),
    };

    let res = match chan.send(Channel::PTY, req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcflow", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcflow", err),
        _ => generic_error("tcflow", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcFlowAction, TcFlowCall},
            Fd,
        },
    };

    use super::tcflow_chan;

    #[test]
    fn test_tcflow() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::Flow(TcFlowCall {
                action: TcFlowAction::TCION,
            }),
        };
        let mock_res = PtySlaveResponse::Success(1);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = tcflow_chan(mock.chan.clone(), 1, libc::TCION);

        assert_eq!(res, 1);
    }
}
