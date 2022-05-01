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

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetattr.html
#[no_mangle]
pub extern "C" fn tcgetattr(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int {
    handle_intercept(
        format!("tcgetattr({})", fd),
        fd,
        |chan| tcgetattr_chan(chan, fd, term),
        || unsafe { __libc__tcgetattr(fd, term) },
    )
}

#[cfg(all(not(test), target_env = "musl"))]
extern "C" {
    // symbol overridden during build scripts
    fn __libc__tcgetattr(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int;
}

#[cfg(any(test, target_os = "macos", target_env = "gnu"))]
#[no_mangle]
#[allow(non_snake_case)]
unsafe fn __libc__tcgetattr(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int {
    let tcgetattr = libc::dlsym(libc::RTLD_NEXT, "tcgetattr\0".as_ptr() as *const _);

    if tcgetattr.is_null() {
        panic!("unable to find tcgetattr sym");
    }

    let tcgetattr = std::mem::transmute::<_, unsafe extern "C" fn(fd: libc::c_int, term: *mut libc::termios) -> libc::c_int>(tcgetattr);

    tcgetattr(fd, term)
}

pub(crate) fn tcgetattr_chan(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    term: *mut libc::termios,
) -> libc::c_int {
    // send tcgetattr request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::GetAttr,
    };

    let res = match chan.send(Channel::PTY, req) {
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

    res.ret as _
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcGetAttrResponse},
            Fd, Termios,
        },
    };

    use super::tcgetattr_chan;

    #[test]
    fn test_tcgetattr() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetAttr,
        };
        let mock_termios = Termios::from_libc_termios(&Termios::zeroed_libc_termios());
        let mock_res = PtySlaveResponse::GetAttr(TcGetAttrResponse {
            ret: 0,
            termios: mock_termios,
        });

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let mut termios = Termios::zeroed_libc_termios();

        let res = tcgetattr_chan(mock.chan.clone(), 1, &mut termios as *mut libc::termios);

        assert_eq!(res, 0);
        assert_eq!(termios.c_iflag, 0);
        assert_eq!(termios.c_oflag, 0);
        assert_eq!(termios.c_cflag, 0);
        assert_eq!(termios.c_lflag, 0);
        #[cfg(target_os = "linux")]
        assert_eq!(termios.c_line, 0);
        assert_eq!(termios.c_cc, [0; libc::NCCS]);
        #[cfg(any(target_env = "gnu", target_os = "macos"))]
        {
            assert_eq!(termios.c_ispeed, 0);
            assert_eq!(termios.c_ospeed, 0);
        }
        #[cfg(target_env = "musl")]
        {
            assert_eq!(termios.__c_ispeed, 0);
            assert_eq!(termios.__c_ospeed, 0);
        }
    }
}
