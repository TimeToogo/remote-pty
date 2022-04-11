use remote_pty_common::proto::{
    slave::{PtySlaveResponse, TcGetAttrResponse},
    Termios,
};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcgetattr.html
pub fn handle_tcgetattr(ctx: &Context) -> PtySlaveResponse {
    let mut termios = Termios::zeroed_libc_termios();

    let ret = unsafe { libc::tcgetattr(ctx.pty.master as _, &mut termios as *mut _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    let termios = Termios::from_libc_termios(&termios);

    PtySlaveResponse::GetAttr(TcGetAttrResponse {
        ret: ret as _,
        termios,
    })
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse, TcError};

    use crate::context::Context;

    use super::handle_tcgetattr;

    #[test]
    fn test_tcgetattr_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let ret = handle_tcgetattr(&ctx);

        let res = match ret {
            PtySlaveResponse::GetAttr(res) => res,
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        };

        assert_eq!(res.ret, 0);
    }

    #[test]
    fn test_tcgetattr_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let ret = handle_tcgetattr(&ctx);

        match ret {
            PtySlaveResponse::Error(err) => assert_eq!(err, TcError::EBADF),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
