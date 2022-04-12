use remote_pty_common::proto::slave::{
    IoctlCall, IoctlResponse, IoctlValueResponse, PtySlaveResponse,
};

use crate::context::Context;

use super::common::handle_error;

// @see https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
pub fn handle_ioctl(ctx: &Context, req: IoctlCall) -> PtySlaveResponse {
    match req {
        IoctlCall::FIONREAD | IoctlCall::TIOCOUTQ | IoctlCall::TIOCGETD => ioctl_get_int(ctx, req),
        IoctlCall::TIOCSETD(_) => ioctl_set_int(ctx, req),
    }
}

pub fn ioctl_get_int(ctx: &Context, req: IoctlCall) -> PtySlaveResponse {
    let cmd = match req {
        IoctlCall::FIONREAD => libc::FIONREAD,
        IoctlCall::TIOCOUTQ => libc::TIOCOUTQ,
        IoctlCall::TIOCGETD => libc::TIOCGETD,
        _ => unreachable!(),
    };

    let mut res = 0 as libc::c_int;
    let ret = unsafe { libc::ioctl(ctx.pty.master as _, cmd as _, &mut res as *mut _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Ioctl(IoctlResponse {
        ret: ret as _,
        val: IoctlValueResponse::Int(res as _),
    })
}

pub fn ioctl_set_int(ctx: &Context, req: IoctlCall) -> PtySlaveResponse {
    let (cmd, d) = match req {
        IoctlCall::TIOCSETD(d) => (libc::TIOCSETD, d),
        _ => unreachable!(),
    };

    let ret = unsafe { libc::ioctl(ctx.pty.master as _, cmd as _, &d as *const _) };

    if ret != 0 {
        return handle_error(ctx);
    }

    PtySlaveResponse::Success(ret as _)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{
        IoctlCall, IoctlResponse, IoctlValueResponse, PtySlaveResponse, TcError,
    };

    use crate::{context::Context, handler::handle_ioctl};

    #[test]
    fn test_ioctl_get_int_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let req = IoctlCall::FIONREAD;
        let ret = handle_ioctl(&ctx, req);

        match ret {
            PtySlaveResponse::Ioctl(IoctlResponse {
                ret,
                val: IoctlValueResponse::Int(v),
            }) => {
                assert_eq!(ret, 0);
                assert_eq!(v, 0);
            }
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_ioctl_get_int_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let req = IoctlCall::FIONREAD;
        let ret = handle_ioctl(&ctx, req);

        match ret {
            PtySlaveResponse::Error(TcError::EBADF) => {}
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_ioctl_set_int_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let req = IoctlCall::TIOCSETD(2);
        let ret = handle_ioctl(&ctx, req);

        match ret {
            PtySlaveResponse::Success(ret) => assert_eq!(ret, 0),
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_ioctl_set_int_with_invalid_fd() {
        let ctx = Context::invalid_fds();
        let req = IoctlCall::TIOCSETD(2);
        let ret = handle_ioctl(&ctx, req);

        match ret {
            PtySlaveResponse::Error(TcError::EBADF) => {}
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        }
    }
}
