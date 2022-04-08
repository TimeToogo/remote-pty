use std::sync::Arc;

use remote_pty_common::log::debug;

use crate::{channel::RemoteChannel, common::handle_intercept};

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetsid.html
#[no_mangle]
pub extern "C" fn remote_tcgetsid(fd: libc::c_int) -> libc::c_int {
    handle_intercept(
        "tcgetsid",
        fd,
        |chan| tcgetsid_chan(chan, fd),
        || unsafe { libc::tcgetsid(fd) },
    )
}

pub(crate) fn tcgetsid_chan(_chan: Arc<dyn RemoteChannel>, _fd: libc::c_int) -> libc::c_int {
    debug("tcgetsid not implemented");
    return -1;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{channel::mock::MockChannel, tcgetsid::tcgetsid_chan};

    #[test]
    fn test_tcgetattr() {
        let chan = MockChannel::new(vec![], vec![]);

        let res = tcgetsid_chan(Arc::new(chan), 1);

        assert_eq!(res, -1);
    }
}
