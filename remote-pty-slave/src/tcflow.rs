use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveResponse, TcFlowAction, TcFlowCall},
    Fd,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcflow.html
#[no_mangle]
pub extern "C" fn tcflow(fd: libc::c_int, action: libc::c_int) -> libc::c_int {
    handle_intercept(
        "tcflow",
        fd,
        |chan| tcflow_chan(chan, fd, action),
        || unsafe { libc::tcflow(fd, action) },
    )
}

fn tcflow_chan(chan: Arc<dyn RemoteChannel>, fd: libc::c_int, action: libc::c_int) -> libc::c_int {
    let action = match action {
        libc::TCOON => TcFlowAction::TCOON,
        libc::TCOOFF => TcFlowAction::TCOOFF,
        libc::TCIOFF => TcFlowAction::TCIOFF,
        libc::TCION => TcFlowAction::TCION,
        _ => return generic_error("tcflow", format!("invalid action {}", action)),
    };

    // send tcflow request to remote
    let req = PtySlaveCall::Flow(TcFlowCall { fd: Fd(fd), action });

    let res = match chan.send(req) {
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
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcFlowAction, TcFlowCall},
        Fd,
    };

    use crate::channel::mock::MockChannel;

    use super::tcflow_chan;

    #[test]
    fn test_tcflow() {
        let expected_req = PtySlaveCall::Flow(TcFlowCall {
            fd: Fd(1),
            action: TcFlowAction::TCION,
        });
        let mock_res = PtySlaveResponse::Success(1);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = tcflow_chan(Arc::new(chan), 1, libc::TCION);

        assert_eq!(res, 1);
    }
}
