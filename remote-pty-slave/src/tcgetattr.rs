use std::sync::Arc;

use remote_pty_common::{
    proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcGetAttrCall},
        Fd,
    },
};

use crate::{
    channel::{get_remote_channel, RemoteChannel},
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetattr.html
#[no_mangle]
pub extern "C" fn tcgetattr(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int {
    let chan = match get_remote_channel() {
        Ok(chan) => chan,
        Err(msg) => return generic_error("tcgetattr", msg),
    };

    tcgetattr_chan(chan, fd, term)
}

fn tcgetattr_chan<C>(chan: Arc<C>, fd: libc::c_int, term: *mut libc::termios) -> libc::c_int
where
    C: RemoteChannel,
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
        (*term).c_ispeed = remote_term.termios.c_ispeed as libc::speed_t;
        (*term).c_ospeed = remote_term.termios.c_ospeed as libc::speed_t;
        #[cfg(target_os = "linux")]
        {
            (*term).c_line = remote_term.termios.c_line as libc::cc_t;
        }
    }

    return 0;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, TcGetAttrCall, PtySlaveResponse, TcGetAttrResponse},
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
            c_ospeed: 7
        };
        let mock_res = PtySlaveResponse::GetAttr(TcGetAttrResponse { termios: mock_termios });

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let mut termios = libc::termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_cc: [0; libc::NCCS],
            c_ispeed: 0,
            c_ospeed: 0,
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
        assert_eq!(termios.c_ispeed, 6);
        assert_eq!(termios.c_ospeed, 7);
    }
}
