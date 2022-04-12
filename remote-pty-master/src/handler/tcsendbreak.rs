use remote_pty_common::proto::slave::{PtySlaveResponse, TcSendBreakCall};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcsendbreak.html
pub fn handle_tcsendbreak(ctx: &Context, req: TcSendBreakCall) -> PtySlaveResponse {
    let ret = unsafe { libc::tcsendbreak(ctx.pty.master as _, req.duration as _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{
        PtySlaveResponse, TcError, TcSendBreakCall, 
    };

    use crate::{context::Context, handler::handle_tcsendbreak};

    #[test]
    fn test_tcsendbreak_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let req = TcSendBreakCall {
            duration: 1,
        };
        let ret = handle_tcsendbreak(&ctx, req);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_tcsendbreak_with_valid_fd_not_pty() {
        let ctx = Context::not_pty_fds();
        let req = TcSendBreakCall {
            duration: 1,
        };
        let ret = handle_tcsendbreak(&ctx, req);

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
    fn test_tcsendbreak_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let req = TcSendBreakCall {
            duration: 1,
        };
        let ret = handle_tcsendbreak(&ctx, req);

        match ret {
            PtySlaveResponse::Error(TcError::EBADF) => {}
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
