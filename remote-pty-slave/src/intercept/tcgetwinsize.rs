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

use super::ioctl;

// non-standard but equivalent to ioctl(fd, TIOCGWINSZ, *winsize)
// @see https://fossies.org/dox/musl-1.2.2/tcgetwinsize_8c_source.html
#[no_mangle]
pub extern "C" fn tcgetwinsize(
    fd: libc::c_int,
    winsize: *mut libc::winsize,
) -> libc::c_int {
    handle_intercept(
        format!("tcgetwinsize({})", fd),
        fd,
        |chan| tcgetwinsize_chan(chan, fd, winsize),
        || ioctl(fd, libc::TIOCGWINSZ, winsize as *mut _),
    )
}

pub(crate) fn tcgetwinsize_chan(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    winsize: *mut libc::winsize,
) -> libc::c_int {
    // send tcgetwinsize request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::GetWinSize,
    };

    let res = match chan.send(Channel::PTY, req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcgetwinsize", msg),
    };

    let remote_winsize = match res {
        PtySlaveResponse::GetWinSize(res) => res,
        PtySlaveResponse::Error(err) => return tc_error("tcgetwinsize", err),
        _ => return generic_error("tcgetwinsize", "unexpected response"),
    };

    // map remote winsize back to local winsize
    unsafe {
        (*winsize).ws_col = remote_winsize.winsize.ws_col;
        (*winsize).ws_row = remote_winsize.winsize.ws_row;
        (*winsize).ws_xpixel = remote_winsize.winsize.ws_xpixel;
        (*winsize).ws_ypixel = remote_winsize.winsize.ws_ypixel;
    }

    remote_winsize.ret as _
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcGetWinSizeResponse},
            Fd, WinSize,
        },
    };

    use super::tcgetwinsize_chan;

    #[test]
    fn test_tcgetwinsize() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetWinSize,
        };
        let mock_winsize = WinSize {
            ws_col: 300,
            ws_row: 80,
            ws_xpixel: 1,
            ws_ypixel: 2,
        };
        let mock_res = PtySlaveResponse::GetWinSize(TcGetWinSizeResponse {
            ret: 0,
            winsize: mock_winsize,
        });
        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let mut winsize = libc::winsize {
            ws_col: 0,
            ws_row: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let res = tcgetwinsize_chan(mock.chan.clone(), 1, &mut winsize as *mut libc::winsize);

        assert_eq!(res, 0);
        assert_eq!(winsize.ws_col, 300);
        assert_eq!(winsize.ws_row, 80);
        assert_eq!(winsize.ws_xpixel, 1);
        assert_eq!(winsize.ws_ypixel, 2);
    }
}
