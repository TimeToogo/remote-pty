use std::sync::Arc;

use remote_pty_common::proto::{
    slave::{PtySlaveCall, PtySlaveResponse, TcSetAttrActions, TcSetAttrCall},
    Fd, Termios,
};

use crate::{
    channel::RemoteChannel,
    common::handle_intercept,
    err::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcsetattr.html
#[no_mangle]
pub extern "C" fn remote_tcsetattr(
    fd: libc::c_int,
    optional_actions: libc::c_int,
    term: *mut libc::termios,
) -> libc::c_int {
    handle_intercept(
        "tcsetattr",
        fd,
        |chan| tcsetattr_chan(chan, fd, optional_actions, term),
        || unsafe { libc::tcsetattr(fd, optional_actions, term) },
    )
}

pub(crate) fn tcsetattr_chan(
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    optional_actions: libc::c_int,
    term: *mut libc::termios,
) -> libc::c_int {
    let optional_actions = match optional_actions {
        libc::TCSANOW => TcSetAttrActions::TCSANOW,
        libc::TCSADRAIN => TcSetAttrActions::TCSADRAIN,
        libc::TCSAFLUSH => TcSetAttrActions::TCSAFLUSH,
        _ => {
            return generic_error(
                "tcsetattr",
                format!("unknown value for optional actions: {}", optional_actions),
            )
        }
    };

    let termios = unsafe {
        #[allow(unused_mut, unused_assignments)]
        let mut c_line = 0;
        #[cfg(target_os = "linux")]
        {
            c_line = (*term).c_line as _;
        }

        let mut c_cc = (*term).c_cc.to_vec();
        c_cc.resize(32, 0);

        Termios {
            c_iflag: (*term).c_iflag as _,
            c_oflag: (*term).c_oflag as _,
            c_cflag: (*term).c_cflag as _,
            c_lflag: (*term).c_lflag as _,
            c_line,
            c_cc: c_cc.try_into().expect("invalid cc length"),
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ispeed: (*term).c_ispeed as _,
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ospeed: (*term).c_ospeed as _,
            #[cfg(target_env = "musl")]
            c_ispeed: (*term).__c_ispeed as _,
            #[cfg(target_env = "musl")]
            c_ospeed: (*term).__c_ospeed as _,
        }
    };

    // send tcsetattr request to remote
    let req = PtySlaveCall::SetAttr(TcSetAttrCall {
        fd: Fd(fd),
        optional_actions,
        termios,
    });

    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcsetattr", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcsetattr", err),
        _ => generic_error("tcsetattr", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcSetAttrActions, TcSetAttrCall},
        Fd, Termios,
    };

    use crate::{channel::mock::MockChannel, tcsetattr::tcsetattr_chan};

    #[test]
    fn test_tcsetattr() {
        let mock_termios = Termios {
            c_iflag: 1,
            c_oflag: 2,
            c_cflag: 3,
            c_lflag: 4,
            #[cfg(target_os = "linux")]
            c_line: 5,
            #[cfg(not(target_os = "linux"))]
            c_line: 0,
            c_cc: [0; 32],
            c_ispeed: 6,
            c_ospeed: 7,
        };
        let expected_req = PtySlaveCall::SetAttr(TcSetAttrCall {
            fd: Fd(1),
            optional_actions: TcSetAttrActions::TCSANOW,
            termios: mock_termios.clone(),
        });
        let mock_res = PtySlaveResponse::Success(0);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let mut termios = libc::termios {
            c_iflag: 1,
            c_oflag: 2,
            c_cflag: 3,
            c_lflag: 4,
            #[cfg(target_os = "linux")]
            c_line: 5,
            c_cc: [0; libc::NCCS],
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ispeed: 6,
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ospeed: 7,
            #[cfg(target_env = "musl")]
            __c_ispeed: 6,
            #[cfg(target_env = "musl")]
            __c_ospeed: 7,
        };

        let res = tcsetattr_chan(
            Arc::new(chan),
            1,
            libc::TCSANOW,
            &mut termios as *mut libc::termios,
        );

        assert_eq!(res, 0);
    }
}
