use std::sync::Arc;

use errno::{set_errno, Errno};
use remote_pty_common::{
    log::debug,
    proto::{
        slave::{
            IoctlCall, IoctlResponse, IoctlValueResponse, PtySlaveCall,
            PtySlaveCallType, PtySlaveResponse,
        },
        Fd,
    },
};

use crate::{channel::RemoteChannel, common::handle_intercept, err::generic_error};

#[cfg(target_os = "linux")]
type Cmd = libc::Ioctl;
#[cfg(not(target_os = "linux"))]
type Cmd = libc::c_ulong;

// @see https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/ioctl.html
// @see https://code.woboq.org/userspace/glibc/sysdeps/unix/sysv/linux/powerpc/ioctl.c.html
#[no_mangle]
pub extern "C" fn intercept_ioctl(
    fd: libc::c_int,
    cmd: Cmd,
    arg: *mut libc::c_void,
) -> libc::c_int {
    handle_intercept(
        "ioctl",
        fd,
        |chan| ioctl_chan(chan, fd, cmd, arg),
        || unsafe { libc::ioctl(fd, cmd, arg) },
    )
}

fn ioctl_chan(
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    cmd: Cmd,
    arg: *mut libc::c_void,
) -> libc::c_int {
    // match against terminal ioctl cmd's

    // check linux specific cmd's
    #[cfg(target_os = "linux")]
    match cmd {
        libc::TCGETS => return crate::tcgetattr_chan(chan, fd, arg as *mut libc::termios),
        libc::TCSETS => {
            return crate::tcsetattr_chan(chan, fd, libc::TCSANOW, arg as *mut libc::termios)
        }
        libc::TCSETSW => {
            return crate::tcsetattr_chan(chan, fd, libc::TCSADRAIN, arg as *mut libc::termios)
        }
        libc::TCSETSF => {
            return crate::tcsetattr_chan(chan, fd, libc::TCSAFLUSH, arg as *mut libc::termios)
        }
        libc::TIOCGLCKTRMIOS => return cmd_unimplemented("TIOCGLCKTRMIOS"),
        libc::TIOCSLCKTRMIOS => return cmd_unimplemented("TIOCSLCKTRMIOS"),
        libc::TCSBRK => return crate::tcsendbreak_chan(chan, fd, arg as libc::c_int),
        libc::TCSBRKP => return crate::tcsendbreak_chan(chan, fd, arg as libc::c_int),
        libc::TCXONC => return crate::tcflow_chan(chan, fd, arg as libc::c_int),
        libc::TIOCINQ => return ioctl_get_int(chan, fd, IoctlCall::FIONREAD, arg), // same as libc::FIONREAD
        libc::TCFLSH => return crate::tcflush_chan(chan, fd, arg as libc::c_int),
        libc::TIOCGSID => return cmd_unimplemented("TIOCGSID"),
        libc::TIOCGEXCL => return cmd_unimplemented("TIOCGEXCL"),
        libc::TIOCGPKT => return cmd_unimplemented("TIOCGPKT"),
        libc::TIOCSPTLCK => return cmd_unimplemented("TIOCSPTLCK"),
        libc::TIOCGPTLCK => return cmd_unimplemented("TIOCGPTLCK"),
        libc::TIOCGPTPEER => return cmd_unimplemented("TIOCGPTPEER"),
        libc::TIOCMIWAIT => return cmd_unimplemented("TIOCMIWAIT"),
        libc::TIOCGICOUNT => return cmd_unimplemented("TIOCGICOUNT"),
        libc::TIOCGSOFTCAR => return cmd_unimplemented("TIOCGSOFTCAR"),
        libc::TIOCSSOFTCAR => return cmd_unimplemented("TIOCSSOFTCAR"),
        _ if cmd == libc::TIOCLINUX as _ => return cmd_unimplemented("TIOCLINUX"),
        _ => {}
    };

    match cmd {
        _ if cmd == libc::TIOCGWINSZ as _ => {
            crate::tcgetwinsize_chan(chan, fd, arg as *mut libc::winsize)
        }
        _ if cmd == libc::TIOCSWINSZ as _ => {
            crate::tcsetwinsize_chan(chan, fd, arg as *mut libc::winsize)
        }
        _ if cmd == libc::TIOCSBRK as _ => cmd_unimplemented("TIOCSBRK"),
        _ if cmd == libc::TIOCCBRK as _ => cmd_unimplemented("TIOCCBRK"),
        _ if cmd == libc::FIONREAD as _ => ioctl_get_int(chan, fd, IoctlCall::FIONREAD, arg),
        _ if cmd == libc::TIOCOUTQ as _ => ioctl_get_int(chan, fd, IoctlCall::TIOCOUTQ, arg),
        _ if cmd == libc::TIOCSTI as _ => cmd_unimplemented("TIOCSTI"),
        _ if cmd == libc::TIOCCONS as _ => cmd_unimplemented("TIOCCONS"),
        _ if cmd == libc::TIOCSCTTY as _ => cmd_unimplemented("TIOCSCTTY"),
        _ if cmd == libc::TIOCNOTTY as _ => cmd_unimplemented("TIOCNOTTY"),
        _ if cmd == libc::TIOCGPGRP as _ => cmd_unimplemented("TIOCGPGRP"),
        _ if cmd == libc::TIOCSPGRP as _ => cmd_unimplemented("TIOCSPGRP"),
        _ if cmd == libc::TIOCEXCL as _ => cmd_unimplemented("TIOCEXCL"),
        _ if cmd == libc::TIOCNXCL as _ => cmd_unimplemented("TIOCNXCL"),
        _ if cmd == libc::TIOCGETD as _ => ioctl_get_int(chan, fd, IoctlCall::TIOCGETD, arg),
        _ if cmd == libc::TIOCSETD as _ => ioctl_set_int(
            chan,
            fd,
            IoctlCall::TIOCSETD(unsafe { *(arg as *const libc::c_int) } as _),
        ),
        _ if cmd == libc::TIOCPKT as _ => cmd_unimplemented("TIOCPKT"),
        _ if cmd == libc::TIOCMGET as _ => cmd_unimplemented("TIOCMGET"),
        _ if cmd == libc::TIOCMSET as _ => cmd_unimplemented("TIOCMSET"),
        _ if cmd == libc::TIOCMBIC as _ => cmd_unimplemented("TIOCMBIC"),
        _ if cmd == libc::TIOCMBIS as _ => cmd_unimplemented("TIOCMBIS"),
        _ => unsafe { libc::ioctl(fd, cmd, arg) },
    }
}

fn ioctl_get_int(
    chan: Arc<dyn RemoteChannel>,
    fd: libc::c_int,
    cmd: IoctlCall,
    arg: *mut libc::c_void,
) -> libc::c_int {
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Ioctl(cmd),
    };

    // send ioctl request to remote
    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("ioctl", msg),
    };

    let (ret, val) = match res {
        PtySlaveResponse::Ioctl(IoctlResponse {
            ret,
            val: IoctlValueResponse::Int(val),
        }) => (ret, val),
        _ => return generic_error("ioctl", "unexpected response"),
    };

    unsafe {
        (*(arg as *mut libc::c_int)) = val as _;
    }

    return ret as _;
}

fn ioctl_set_int(chan: Arc<dyn RemoteChannel>, fd: libc::c_int, cmd: IoctlCall) -> libc::c_int {
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Ioctl(cmd),
    };

    // send ioctl request to remote
    let res = match chan.send(req) {
        Ok(res) => res,
        Err(msg) => return generic_error("ioctl", msg),
    };

    match res {
        PtySlaveResponse::Success(ret) => ret as _,
        _ => generic_error("ioctl", "unexpected response"),
    }
}

fn cmd_unimplemented(name: &str) -> libc::c_int {
    debug(format!("unimplemented ioctl {}", name));
    set_errno(Errno(libc::EINVAL));
    return -1;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use remote_pty_common::proto::{
        slave::{
            IoctlCall, IoctlResponse, IoctlValueResponse, PtySlaveCall, PtySlaveCallType,
            PtySlaveResponse,
        },
        Fd,
    };

    use crate::{channel::mock::MockChannel, ioctl::ioctl_chan};

    #[test]
    fn test_unimplemented() {
        let chan = MockChannel::new(vec![], vec![]);

        let res = ioctl_chan(
            Arc::new(chan),
            1,
            libc::TIOCSBRK as _,
            &mut 10 as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, -1);
        assert_eq!(errno::errno().0, libc::EINVAL);
    }

    #[test]
    fn test_non_terminal_ioctl() {
        let chan = MockChannel::new(vec![], vec![]);

        let res = ioctl_chan(
            Arc::new(chan),
            100,
            libc::SIOCGIFADDR as _,
            &mut 10 as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, -1);
        assert_eq!(errno::errno().0, libc::EBADF);
    }

    #[test]
    fn test_ioctl_tiocsetd() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::Ioctl(IoctlCall::TIOCSETD(10)),
        };
        let mock_res = PtySlaveResponse::Success(1);

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);

        let res = ioctl_chan(
            Arc::new(chan),
            1,
            libc::TIOCSETD,
            &mut 10 as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 1);
    }

    #[test]
    fn test_ioctl_tiocgetd() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::Ioctl(IoctlCall::TIOCGETD),
        };
        let mock_res = PtySlaveResponse::Ioctl(IoctlResponse {
            ret: 0,
            val: IoctlValueResponse::Int(5),
        });

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::c_int) as *mut libc::c_int;

        let res = ioctl_chan(
            Arc::new(chan),
            1,
            libc::TIOCGETD,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
        assert_eq!(unsafe { *val }, 5 as libc::c_int);
    }

    #[test]
    fn test_ioctl_fionread() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::Ioctl(IoctlCall::FIONREAD),
        };
        let mock_res = PtySlaveResponse::Ioctl(IoctlResponse {
            ret: 0,
            val: IoctlValueResponse::Int(10),
        });

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::c_int) as *mut libc::c_int;

        let res = ioctl_chan(
            Arc::new(chan),
            1,
            libc::FIONREAD,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
        assert_eq!(unsafe { *val }, 10 as libc::c_int);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ioctl_tiocinq() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::Ioctl(IoctlCall::FIONREAD),
        };
        let mock_res = PtySlaveResponse::Ioctl(IoctlResponse {
            ret: 0,
            val: IoctlValueResponse::Int(10),
        });

        let chan = MockChannel::new(vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::c_int) as *mut libc::c_int;

        let res = ioctl_chan(
            Arc::new(chan),
            1,
            libc::TIOCINQ,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
        assert_eq!(unsafe { *val }, 10 as libc::c_int);
    }
}
