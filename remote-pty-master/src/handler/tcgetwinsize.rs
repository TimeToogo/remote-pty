use remote_pty_common::proto::{
    slave::{PtySlaveResponse, TcGetWinSizeResponse},
    WinSize,
};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/ioctl.html
pub fn handle_tcgetwinsize(ctx: &Context) -> PtySlaveResponse {
    let mut winsize = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let ret = unsafe {
        libc::ioctl(
            ctx.pty.master as _,
            libc::TIOCGWINSZ,
            &mut winsize as *mut _,
        )
    };

    if ret == -1 {
        return handle_error(ctx);
    }

    let winsize = WinSize {
        ws_row: winsize.ws_row as _,
        ws_col: winsize.ws_col as _,
        ws_xpixel: winsize.ws_xpixel as _,
        ws_ypixel: winsize.ws_ypixel as _,
    };

    PtySlaveResponse::GetWinSize(TcGetWinSizeResponse {
        ret: ret as _,
        winsize,
    })
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse, TcError};

    use crate::{context::Context, handler::handle_tcgetwinsize};

    #[test]
    fn test_tcgetwinsize_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let ret = handle_tcgetwinsize(&ctx);

        let res = match ret {
            PtySlaveResponse::GetWinSize(res) => res,
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        };

        assert_eq!(res.ret, 0);
    }

    #[test]
    fn test_tcgetwinsize_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let ret = handle_tcgetwinsize(&ctx);

        match ret {
            PtySlaveResponse::Error(err) => assert_eq!(err, TcError::EBADF),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
