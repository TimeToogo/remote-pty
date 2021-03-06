use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{
            PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcSetAttrActions, TcSetAttrCall,
        },
        Fd, Termios,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcsetattr.html
#[no_mangle]
pub extern "C" fn tcsetattr(
    fd: libc::c_int,
    optional_actions: libc::c_int,
    term: *mut libc::termios,
) -> libc::c_int {
    handle_intercept(
        format!("tcsetattr({}, ...)", fd),
        fd,
        |chan| tcsetattr_chan(chan, fd, optional_actions, term),
        || unsafe { __libc__tcsetattr(fd, optional_actions, term) },
    )
}

#[cfg(all(not(test), target_env = "musl"))]
extern "C" {
    // symbol overridden during build scripts
    fn __libc__tcsetattr(fd: libc::c_int, optional_actions: libc::c_int, term: *mut libc::termios) -> libc::c_int;
}

#[cfg(any(test, target_os = "macos", target_env = "gnu"))]
#[no_mangle]
#[allow(non_snake_case)]
unsafe fn __libc__tcsetattr(fd: libc::c_int, optional_actions: libc::c_int, term: *mut libc::termios) -> libc::c_int {
    let tcsetattr = libc::dlsym(libc::RTLD_NEXT, "tcsetattr\0".as_ptr() as *const _);

    if tcsetattr.is_null() {
        panic!("unable to find tcsetattr sym");
    }

    let tcsetattr = std::mem::transmute::<_, unsafe extern "C" fn(fd: libc::c_int, optional_actions: libc::c_int, term: *mut libc::termios) -> libc::c_int>(tcsetattr);

    tcsetattr(fd, optional_actions, term)
}

pub(crate) fn tcsetattr_chan(
    mut chan: RemoteChannel,
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

    let termios = unsafe { Termios::from_libc_termios(term.as_ref().unwrap()) };

    // send tcsetattr request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::SetAttr(TcSetAttrCall {
            optional_actions,
            termios,
        }),
    };

    let res = match chan.send(Channel::PTY, req) {
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
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{
                PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcSetAttrActions, TcSetAttrCall,
            },
            Fd, Termios,
        },
    };

    use crate::intercept::tcsetattr_chan;

    #[test]
    fn test_tcsetattr() {
        let mut mock_termios = libc::termios {
            c_iflag: libc::IGNBRK,
            c_oflag: libc::OCRNL,
            c_cflag: libc::CREAD,
            c_lflag: libc::ISIG,
            #[cfg(target_os = "linux")]
            c_line: 0,
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
        
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::SetAttr(TcSetAttrCall {
                optional_actions: TcSetAttrActions::TCSANOW,
                termios: Termios::from_libc_termios(&mock_termios),
            }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = tcsetattr_chan(
            mock.chan.clone(),
            1,
            libc::TCSANOW,
            &mut mock_termios as *mut libc::termios,
        );

        assert_eq!(res, 0);
    }
}
