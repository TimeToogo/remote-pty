use remote_pty_common::proto::slave::PtySlaveResponse;

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/isatty.html
pub fn handle_isatty(ctx: &Context) -> PtySlaveResponse {
    let ret = unsafe { libc::isatty(ctx.pty.master as _) };

    if ret != 1 && errno::errno().0 != libc::ENOTTY {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse, TcError};

    use crate::{context::Context, handler::handle_isatty};

    #[test]
    fn test_isatty_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let ret = handle_isatty(&ctx);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 1),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_isatty_with_valid_fd_not_pty() {
        let ctx = Context::not_pty_fds();
        let ret = handle_isatty(&ctx);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_isatty_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let ret = handle_isatty(&ctx);

        match ret {
            PtySlaveResponse::Error(err) => assert_eq!(err, TcError::EBADF),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
