use remote_pty_common::{
    log::debug,
    proto::slave::{PtySlaveResponse, TcError},
};

use crate::context::Context;

// handle libc errno results and convert to response messages
pub fn handle_error(_ctx: &Context) -> PtySlaveResponse {
    let err = match errno::errno().0 as _ {
        libc::EINVAL => TcError::EINVAL,
        libc::EBADF => TcError::EBADF,
        libc::ENOTTY => TcError::ENOTTY,
        libc::EINTR => TcError::EINTR,
        libc::EIO => TcError::EIO,
        _ => {
            debug(format!("unknown libc errno {}", errno::errno()));
            TcError::EINVAL
        }
    };

    PtySlaveResponse::Error(err)
}
