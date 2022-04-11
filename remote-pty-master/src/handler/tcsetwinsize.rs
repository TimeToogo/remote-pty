use remote_pty_common::proto::slave::{PtySlaveResponse, TcSetWinSizeCall};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/ioctl.html
pub fn handle_tcsetwinsize(ctx: &Context, req: TcSetWinSizeCall) -> PtySlaveResponse {
    let ret = unsafe {
        let winsize = libc::winsize {
            ws_row: req.winsize.ws_row as _,
            ws_col: req.winsize.ws_col as _,
            ws_xpixel: req.winsize.ws_xpixel as _,
            ws_ypixel: req.winsize.ws_ypixel as _,
        };

        let ret = libc::ioctl(ctx.pty.master as _, libc::TIOCSWINSZ, &winsize as *const _);

        if ret == -1 {
            return handle_error(ctx);
        }

        ret
    };

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::{
        slave::{PtySlaveResponse, TcError, TcSetWinSizeCall},
        WinSize,
    };

    use crate::{context::Context, handler::handle_tcsetwinsize};

    #[test]
    fn test_tcsetwinsize_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let mock_req = TcSetWinSizeCall {
            winsize: WinSize {
                ws_col: 300,
                ws_row: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        };

        let ret = handle_tcsetwinsize(&ctx, mock_req);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        };
    }

    #[test]
    fn test_tcsetwinsize_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let mock_req = TcSetWinSizeCall {
            winsize: WinSize {
                ws_col: 300,
                ws_row: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        };

        let ret = handle_tcsetwinsize(&ctx, mock_req);

        match ret {
            PtySlaveResponse::Error(err) => assert_eq!(err, TcError::EBADF),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
