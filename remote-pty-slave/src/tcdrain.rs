use std::sync::Arc;

use remote_pty_common::{
    proto::{
        slave::{PtySlaveCall, PtySlaveResponse, PtySlaveCallType},
        Fd,
    },
};

use crate::{
    channel::RemoteChannel,
    err::{generic_error, tc_error}, common::handle_intercept,
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcdrain.html
#[no_mangle]
pub extern "C" fn intercept_tcdrain(fd: libc::c_int) -> libc::c_int {
    handle_intercept(
        format!("tcdrain({})", fd),
        fd,
        |chan| tcdrain_chan(chan, fd),
        || unsafe { libc::tcdrain(fd) },
    )
}

pub(crate) fn tcdrain_chan(chan: Arc<dyn RemoteChannel>, fd: libc::c_int) -> libc::c_int
{
    // send tcdrain request to remote
    let req = PtySlaveCall { fd: Fd(fd), typ: PtySlaveCallType::Drain };

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcdrain", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcdrain", err),
        _ => generic_error("tcdrain", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, PtySlaveCallType},
        Fd,
    };

    use crate::channel::mock::MockChannel;

    use super::tcdrain_chan;

    #[test]
    fn test_tcdrain() {
        let expected_req = PtySlaveCall { fd: Fd(1), typ: PtySlaveCallType::Drain };
        let mock_res = PtySlaveResponse::Success(1);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = tcdrain_chan(Arc::new(chan), 1);

        assert_eq!(res, 1);
    }
}
