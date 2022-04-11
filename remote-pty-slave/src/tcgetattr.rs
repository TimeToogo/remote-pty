use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
    Fd,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetattr.html
#[no_mangle]
pub extern "C" fn intercept_tcgetattr(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int {
    handle_intercept(
        "tcgetattr",
        fd,
        |chan| tcgetattr_chan(chan, fd, term),
        || unsafe { libc::tcgetattr(fd, term) },
    )
}

pub(crate) fn tcgetattr_chan(
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    term: *mut libc::termios,
) -> libc::c_int {
    // send tcgetattr request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::GetAttr,
    };

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcgetattr", msg),
    };

    let res = match res {
        PtySlaveResponse::GetAttr(term) => term,
        PtySlaveResponse::Error(err) => return tc_error("tcgetattr", err),
        _ => return generic_error("tcgetattr", "unexpected response"),
    };

    res.termios
        .copy_to_libc_termios(unsafe { term.as_mut().unwrap() });

    return res.ret as _;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcGetAttrResponse},
        Fd, Termios,
    };

    use crate::channel::mock::MockChannel;

    use super::tcgetattr_chan;

    #[test]
    fn test_tcgetattr() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetAttr,
        };
        let mock_termios = Termios {
            c_iflag: 1,
            c_oflag: 2,
            c_cflag: 3,
            c_lflag: 4,
            c_line: 5,
            c_cc: [1; 32],
            c_ispeed: 6,
            c_ospeed: 7,
        };
        let mock_res = PtySlaveResponse::GetAttr(TcGetAttrResponse {
            ret: 0,
            termios: mock_termios,
        });

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let mut termios = Termios::zeroed_libc_termios();

        let res = tcgetattr_chan(Arc::new(chan), 1, &mut termios as *mut libc::termios);

        assert_eq!(res, 0);
        assert_eq!(termios.c_iflag, 1);
        assert_eq!(termios.c_oflag, 2);
        assert_eq!(termios.c_cflag, 3);
        assert_eq!(termios.c_lflag, 4);
        #[cfg(target_os = "linux")]
        assert_eq!(termios.c_line, 5);
        assert_eq!(termios.c_cc, [1; libc::NCCS]);
        #[cfg(any(target_env = "gnu", target_os = "macos"))]
        {
            assert_eq!(termios.c_ispeed, 6);
            assert_eq!(termios.c_ospeed, 7);
        }
        #[cfg(target_env = "musl")]
        {
            assert_eq!(termios.__c_ispeed, 6);
            assert_eq!(termios.__c_ospeed, 7);
        }
    }
}
