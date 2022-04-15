use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    proto::{
        slave::{
            PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcFlushCall, TcFlushQueueSelector,
        },
        Fd,
    },
};

use crate::{
    common::handle_intercept,
    error::{generic_error, tc_error},
};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcflush.html
#[no_mangle]
pub extern "C" fn tcflush(fd: libc::c_int, queue_selector: libc::c_int) -> libc::c_int {
    handle_intercept(
        format!("tcflush({})", fd),
        fd,
        |chan| tcflush_chan(chan, fd, queue_selector),
        || unsafe { __libc__tcflush(fd, queue_selector) },
    )
}

#[cfg(all(not(test), target_env = "musl"))]
extern "C" {
    // symbol overridden during build scripts
    fn __libc__tcflush(fd: libc::c_int, queue_selector: libc::c_int) -> libc::c_int;
}

#[cfg(any(test, target_os = "macos", target_env = "gnu"))]
#[no_mangle]
#[allow(non_snake_case)]
unsafe fn __libc__tcflush(fd: libc::c_int, queue_selector: libc::c_int) -> libc::c_int {
    let tcflush = libc::dlsym(libc::RTLD_NEXT, "tcflush\0".as_ptr() as *const _);

    if tcflush.is_null() {
        panic!("unable to find tcflush sym");
    }

    let tcflush = std::mem::transmute::<_, unsafe extern "C" fn(fd: libc::c_int, queue_selector: libc::c_int) -> libc::c_int>(tcflush);

    tcflush(fd, queue_selector)
}

pub(crate) fn tcflush_chan(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    queue_selector: libc::c_int,
) -> libc::c_int {
    let queue_selector = match queue_selector {
        libc::TCIFLUSH => TcFlushQueueSelector::TCIFLUSH,
        libc::TCOFLUSH => TcFlushQueueSelector::TCOFLUSH,
        libc::TCIOFLUSH => TcFlushQueueSelector::TCIOFLUSH,
        _ => {
            return generic_error(
                "tcflush",
                format!("invalid queue selector {}", queue_selector),
            )
        }
    };

    // send tcflush request to remote
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Flush(TcFlushCall { queue_selector }),
    };

    let res = match chan.send(Channel::PTY, req) {
        Ok(res) => res,
        Err(msg) => return generic_error("tcflush", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        PtySlaveResponse::Error(err) => tc_error("tcflush", err),
        _ => generic_error("tcflush", "unexpected response"),
    }
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{Channel, mock::MockChannel},
        proto::{
            slave::{PtySlaveCall, PtySlaveResponse, TcFlushCall, TcFlushQueueSelector},
            Fd,
        },
    };

    use super::tcflush_chan;

    #[test]
    fn test_tcflush() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: remote_pty_common::proto::slave::PtySlaveCallType::Flush(TcFlushCall {
                queue_selector: TcFlushQueueSelector::TCIOFLUSH,
            }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = tcflush_chan(mock.chan.clone(), 1, libc::TCIOFLUSH);

        assert_eq!(res, 0);
    }
}
