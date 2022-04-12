use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcSetWinSizeCall},
    Fd, WinSize,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// non-standard but equivalent to ioctl(fd, TCIOSWINSZ, *winsize)
// @see https://fossies.org/dox/musl-1.2.2/tcsetwinsize_8c_source.html
#[no_mangle]
pub extern "C" fn intercept_tcsetwinsize(
    fd: libc::c_int,
    winsize: *mut libc::winsize,
) -> libc::c_int {
    handle_intercept(
        format!("tcsetwinsize({}, ...)", fd),
        fd,
        |chan| tcsetwinsize_chan(chan, fd, winsize),
        || unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, winsize) },
    )
}

pub(crate) fn tcsetwinsize_chan(
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    winsize: *mut libc::winsize,
) -> libc::c_int {
    let remote_winsize = unsafe {
        WinSize {
            ws_col: (*winsize).ws_col as _,
            ws_row: (*winsize).ws_row as _,
            ws_xpixel: (*winsize).ws_xpixel as _,
            ws_ypixel: (*winsize).ws_ypixel as _,
        }
    };

    // send tcsetwinsize request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::SetWinSize(TcSetWinSizeCall {
            winsize: remote_winsize,
        }),
    };

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcsetwinsize", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcsetwinsize", err),
        _ => generic_error("tcsetwinsize", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcSetWinSizeCall, PtySlaveCallType},
        Fd, WinSize,
    };

    use crate::{channel::mock::MockChannel, intercept::tcsetwinsize_chan};

    #[test]
    fn test_tcsetwinsize() {
        let mock_winsize = WinSize {
            ws_col: 300,
            ws_row: 80,
            ws_xpixel: 1,
            ws_ypixel: 2,
        };
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::SetWinSize(TcSetWinSizeCall {
                winsize: mock_winsize,
            }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let mut winsize = libc::winsize {
            ws_col: 300,
            ws_row: 80,
            ws_xpixel: 1,
            ws_ypixel: 2,
        };

        let res = tcsetwinsize_chan(Arc::new(chan), 1, &mut winsize as *mut libc::winsize);

        assert_eq!(res, 0);
    }
}
