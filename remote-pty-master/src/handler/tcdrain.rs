use remote_pty_common::proto::slave::PtySlaveResponse;

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcdrain.html
pub fn handle_tcdrain(ctx: &Context) -> PtySlaveResponse {
    let ret = unsafe { libc::tcdrain(ctx.pty.master as _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse, TcError};

    use crate::{context::Context, handler::handle_tcdrain};

    #[test]
    fn test_tcdrain_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let ret = handle_tcdrain(&ctx);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_tcdrain_with_valid_fd_not_pty() {
        let ctx = Context::not_pty_fds();
        let ret = handle_tcdrain(&ctx);

        match ret {
            PtySlaveResponse::Error(TcError::ENOTTY) => {},
            PtySlaveResponse::Error(TcError::EINVAL) => {},
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_tcdrain_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let ret = handle_tcdrain(&ctx);

        match ret {
            PtySlaveResponse::Error(TcError::EBADF) => {},
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
