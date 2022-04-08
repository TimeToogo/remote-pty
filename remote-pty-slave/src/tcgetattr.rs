use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveResponse, TcGetAttrCall},
    Fd,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetattr.html
#[no_mangle]
pub extern "C" fn remote_tcgetattr(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int {
    handle_intercept(
        "tcgetattr",
        fd,
        |chan| tcgetattr_chan(chan, fd, term),
        || unsafe { libc::tcgetattr(fd, term) },
    )
}

pub(crate) fn tcgetattr_chan(chan: Arc<dyn RemoteChannel>, fd: libc::c_int, term: *mut libc::termios) -> libc::c_int
{
    // send tcgetattr request to remote
    let req = PtySlaveCall::GetAttr(TcGetAttrCall { fd: Fd(fd) });

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcgetattr", msg),
    };

    let remote_term = match res {
        PtySlaveResponse::GetAttr(term) => term,
        PtySlaveResponse::Error(err) => return tc_error("tcgetattr", err),
        _ => return generic_error("tcgetattr", "unexpected response"),
    };

    // map remote termios back to local termios
    // TODO: improve naive mapping
    unsafe {
        (*term).c_iflag = remote_term.termios.c_iflag as libc::tcflag_t;
        (*term).c_oflag = remote_term.termios.c_oflag as libc::tcflag_t;
        (*term).c_cflag = remote_term.termios.c_cflag as libc::tcflag_t;
        (*term).c_lflag = remote_term.termios.c_lflag as libc::tcflag_t;
        (*term)
            .c_cc
            .copy_from_slice(&remote_term.termios.c_cc[..libc::NCCS]);
        #[cfg(target_os = "linux")]
        {
            (*term).c_line = remote_term.termios.c_line as libc::cc_t;
        }
        #[cfg(any(target_env = "gnu", target_os = "macos"))]
        {
            (*term).c_ispeed = remote_term.termios.c_ispeed as libc::speed_t;
            (*term).c_ospeed = remote_term.termios.c_ospeed as libc::speed_t;
        }
        #[cfg(target_env = "musl")]
        {
            (*term).__c_ispeed = remote_term.termios.c_ispeed as libc::speed_t;
            (*term).__c_ospeed = remote_term.termios.c_ospeed as libc::speed_t;
        }
    }

    return remote_term.ret as _;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcGetAttrCall, TcGetAttrResponse},
        Fd, Termios,
    };

    use crate::channel::mock::MockChannel;

    use super::tcgetattr_chan;

    #[test]
    fn test_tcgetattr() {
        let expected_req = PtySlaveCall::GetAttr(TcGetAttrCall { fd: Fd(1) });
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

        let mut termios = libc::termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_cc: [0; libc::NCCS],
            #[cfg(target_os = "linux")]
            c_line: 0,
            #[cfg(not(target_os = "linux"))]
            c_ispeed: 0,
            #[cfg(not(target_os = "linux"))]
            c_ospeed: 0,
            #[cfg(target_os = "linux")]
            __c_ispeed: 0,
            #[cfg(target_os = "linux")]
            __c_ospeed: 0,
        };

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
