use remote_pty_common::proto::{
    slave::{PtySlaveResponse, TcSetAttrActions, TcSetAttrCall},
    Termios,
};

use crate::context::Context;

use super::common::handle_error;

// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/tcsetattr.html
pub fn handle_tcsetattr(ctx: &Context, req: TcSetAttrCall) -> PtySlaveResponse {
    let optional_actions = match req.optional_actions {
        TcSetAttrActions::TCSANOW => libc::TCSANOW,
        TcSetAttrActions::TCSADRAIN => libc::TCSADRAIN,
        TcSetAttrActions::TCSAFLUSH => libc::TCSAFLUSH,
    };

    let mut termios = Termios::zeroed_libc_termios();
    req.termios.copy_to_libc_termios(&mut termios);

    let ret = unsafe {
        libc::tcsetattr(
            ctx.pty.master as _,
            optional_actions as _,
            &mut termios as *mut _,
        )
    };

    if ret == -1 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::{
        slave::{PtySlaveResponse, TcError, TcSetAttrActions, TcSetAttrCall},
        Termios,
    };

    use crate::{context::Context, handler::handle_tcsetattr};

    #[test]
    fn test_tcsetattr_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let mock_req = TcSetAttrCall {
            optional_actions: TcSetAttrActions::TCSANOW,
            termios: Termios::from_libc_termios(&Termios::zeroed_libc_termios()),
        };

        let ret = handle_tcsetattr(&ctx, mock_req);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        };
    }

    #[test]
    fn test_tcsetattr_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let mock_req = TcSetAttrCall {
            optional_actions: TcSetAttrActions::TCSANOW,
            termios: Termios::from_libc_termios(&Termios::zeroed_libc_termios()),
        };

        let ret = handle_tcsetattr(&ctx, mock_req);

        match ret {
            PtySlaveResponse::Error(err) => assert_eq!(err, TcError::EBADF),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
