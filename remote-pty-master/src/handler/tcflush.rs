use remote_pty_common::proto::slave::{PtySlaveResponse, TcFlushCall, TcFlushQueueSelector};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcflush.html
pub fn handle_tcflush(ctx: &Context, req: TcFlushCall) -> PtySlaveResponse {
    let queue_selector = match req.queue_selector {
        TcFlushQueueSelector::TCIFLUSH => libc::TCIFLUSH,
        TcFlushQueueSelector::TCOFLUSH => libc::TCOFLUSH,
        TcFlushQueueSelector::TCIOFLUSH => libc::TCIOFLUSH,
    };

    let ret = unsafe { libc::tcflush(ctx.pty.master as _, queue_selector as _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{
        PtySlaveResponse, TcError, TcFlushCall, TcFlushQueueSelector,
    };

    use crate::{context::Context, handler::handle_tcflush};

    #[test]
    fn test_tcflush_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let req = TcFlushCall {
            queue_selector: TcFlushQueueSelector::TCIFLUSH,
        };
        let ret = handle_tcflush(&ctx, req);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_tcflush_with_valid_fd_not_pty() {
        let ctx = Context::not_pty_fds();
        let req = TcFlushCall {
            queue_selector: TcFlushQueueSelector::TCIFLUSH,
        };
        let ret = handle_tcflush(&ctx, req);

        match ret {
            PtySlaveResponse::Error(TcError::ENOTTY) => {}
            PtySlaveResponse::Error(TcError::EINVAL) => {}
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_tcflush_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let req = TcFlushCall {
            queue_selector: TcFlushQueueSelector::TCIFLUSH,
        };
        let ret = handle_tcflush(&ctx, req);

        match ret {
            PtySlaveResponse::Error(TcError::EBADF) => {}
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
