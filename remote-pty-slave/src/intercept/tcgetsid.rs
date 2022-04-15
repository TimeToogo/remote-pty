use remote_pty_common::{channel::RemoteChannel, log::debug};

use crate::common::handle_intercept;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetsid.html
#[no_mangle]
pub extern "C" fn tcgetsid(fd: libc::c_int) -> libc::c_int {
    handle_intercept(
        "tcgetsid",
        fd,
        |chan| tcgetsid_chan(chan, fd),
        || unsafe { __libc__tcgetsid(fd) },
    )
}

#[cfg(all(not(test), target_env = "musl"))]
extern "C" {
    // symbol overridden during build scripts
    fn __libc__tcgetsid(fd: libc::c_int) -> libc::c_int;
}

#[cfg(any(test, target_os = "macos", target_env = "gnu"))]
#[no_mangle]
#[allow(non_snake_case)]
unsafe fn __libc__tcgetsid(fd: libc::c_int) -> libc::c_int {
    let tcgetsid = libc::dlsym(libc::RTLD_NEXT, "tcgetsid\0".as_ptr() as *const _);

    if tcgetsid.is_null() {
        panic!("unable to find tcgetsid sym");
    }

    let tcgetsid = std::mem::transmute::<_, unsafe extern "C" fn(fd: libc::c_int) -> libc::c_int>(tcgetsid);

    tcgetsid(fd)
}

pub(crate) fn tcgetsid_chan(_chan: RemoteChannel, _fd: libc::c_int) -> libc::c_int {
    debug("tcgetsid not implemented");
    -1
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{mock::MockChannel, Channel},
        proto::slave::{PtySlaveCall, PtySlaveResponse},
    };

    use crate::intercept::tcgetsid_chan;

    #[test]
    fn test_tcgetattr() {
        let mock = MockChannel::assert_sends::<PtySlaveCall, PtySlaveResponse>(
            Channel::PTY,
            vec![],
            vec![],
        );

        let res = tcgetsid_chan(mock.chan.clone(), 1);

        assert_eq!(res, -1);
    }
}
