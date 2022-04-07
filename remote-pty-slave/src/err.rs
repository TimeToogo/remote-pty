use errno::{set_errno, Errno};
use remote_pty_common::{proto::slave::TcError, log::debug};

pub fn generic_error(func_name: &str, err_msg: &str) -> libc::c_int {
    debug(format!("{}: {}", func_name, err_msg));
    set_errno(Errno(libc::EIO));
    return -1;
}

pub fn tc_error(func_name: &str, err: TcError) -> libc::c_int {
    debug(format!("{}: {:?}", func_name, err));

    set_errno(Errno(match err {
        TcError::EINVAL => libc::EINVAL,
        TcError::EBADF => libc::EBADF,
        TcError::ENOTTY => libc::ENOTTY,
        TcError::EINTR => libc::EINTR,
        TcError::EIO => libc::EIO,
    }));

    return -1;
}