use remote_pty_common::proto::slave::{PtySlaveResponse, TcFlowCall, TcFlowAction};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcflow.html
pub fn handle_tcflow(ctx: &Context, req: TcFlowCall) -> PtySlaveResponse {
    let action = match req.action {
        TcFlowAction::TCOOFF => libc::TCOOFF,
        TcFlowAction::TCOON => libc::TCOON,
        TcFlowAction::TCIOFF => libc::TCIOFF,
        TcFlowAction::TCION => libc::TCION
    };

    let ret = unsafe { libc::tcflow(ctx.pty.master as _, action as _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse, TcError, TcFlowAction, TcFlowCall};

    use crate::{context::Context, handler::handle_tcflow};

    #[test]
    fn test_tcflow_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let action = TcFlowCall { action: TcFlowAction::TCIOFF };
        let ret = handle_tcflow(&ctx, action);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_tcflow_with_valid_fd_not_pty() {
        let ctx = Context::not_pty_fds();
        let action = TcFlowCall { action: TcFlowAction::TCIOFF };
        let ret = handle_tcflow(&ctx, action);

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
    fn test_tcflow_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let action = TcFlowCall { action: TcFlowAction::TCIOFF };
        let ret = handle_tcflow(&ctx, action);

        match ret {
            PtySlaveResponse::Error(TcError::EBADF) => {},
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
