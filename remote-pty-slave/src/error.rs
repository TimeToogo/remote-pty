use errno::{set_errno, Errno};
use remote_pty_common::{log::debug, proto::slave::TcError};

pub fn generic_error<S1, S2>(func_name: S1, err_msg: S2) -> libc::c_int
where
    S1: Into<String>,
    S2: Into<String>,
{
    debug(format!("{}: {}", func_name.into(), err_msg.into()));
    set_errno(Errno(libc::EIO));
    -1
}

pub fn tc_error<S1>(func_name: S1, err: TcError) -> libc::c_int
where
    S1: Into<String>,
{
    debug(format!("{}: {:?}", func_name.into(), err));

    set_errno(Errno(match err {
        TcError::EINVAL => libc::EINVAL,
        TcError::EBADF => libc::EBADF,
        TcError::ENOTTY => libc::ENOTTY,
        TcError::EINTR => libc::EINTR,
        TcError::EIO => libc::EIO,
    }));

    -1
}
