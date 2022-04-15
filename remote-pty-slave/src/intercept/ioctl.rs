use errno::{set_errno, Errno};
use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::{
        slave::{
            IoctlCall, IoctlResponse, IoctlValueResponse, PtySlaveCall, PtySlaveCallType,
            PtySlaveResponse,
        },
        Fd,
    },
};

use crate::{common::handle_intercept, error::generic_error, intercept};

#[cfg(target_os = "linux")]
type Cmd = libc::Ioctl;
#[cfg(not(target_os = "linux"))]
type Cmd = libc::c_ulong;

// @see https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/ioctl.html
// @see https://code.woboq.org/userspace/glibc/sysdeps/unix/sysv/linux/powerpc/ioctl.c.html
#[no_mangle]
pub extern "C" fn ioctl(fd: libc::c_int, cmd: Cmd, arg: *mut libc::c_void) -> libc::c_int {
    handle_intercept(
        format!("ioctl({}, {}, ...)", fd, cmd),
        fd,
        |chan| ioctl_chan(chan, fd, cmd, arg),
        || unsafe { __libc__ioctl(fd, cmd, arg) },
    )
}

#[cfg(target_env = "musl")]
extern "C" {
    // symbol overridden during build scripts
    fn __libc__ioctl(fd: libc::c_int, cmd: Cmd, arg: *mut libc::c_void) -> libc::c_int;
}

#[cfg(any(test, target_os = "macos", target_env = "gnu"))]
#[no_mangle]
#[allow(non_snake_case)]
unsafe fn __libc__ioctl(fd: libc::c_int, cmd: Cmd, arg: *mut libc::c_void) -> libc::c_int {
    let ioctl = libc::dlsym(libc::RTLD_NEXT, "ioctl\0".as_ptr() as *const _);

    if ioctl.is_null() {
        panic!("unable to find ioctl sym");
    }

    let ioctl = std::mem::transmute::<_, unsafe extern "C" fn(fd: libc::c_int, cmd: Cmd, ...) -> libc::c_int>(ioctl);

    ioctl(fd, cmd, arg)
}

// #[cfg(test)]
// #[no_mangle]
// #[allow(non_snake_case)]
// unsafe fn __libc__ioctl(fd: libc::c_int, cmd: Cmd, arg: *mut libc::c_void) -> libc::c_int {
// libc::ioctl(fd, cmd, arg)
// }

fn ioctl_chan(
    chan: RemoteChannel,
    fd: libc::c_int,
    cmd: Cmd,
    arg: *mut libc::c_void,
) -> libc::c_int {
    // match against terminal ioctl cmd's

    // check linux specific cmd's
    #[cfg(target_os = "linux")]
    match cmd {
        libc::TCGETS => return intercept::tcgetattr_chan(chan, fd, arg as *mut libc::termios),
        libc::TCSETS => {
            return intercept::tcsetattr_chan(chan, fd, libc::TCSANOW, arg as *mut libc::termios)
        }
        libc::TCSETSW => {
            return intercept::tcsetattr_chan(chan, fd, libc::TCSADRAIN, arg as *mut libc::termios)
        }
        libc::TCSETSF => {
            return intercept::tcsetattr_chan(chan, fd, libc::TCSAFLUSH, arg as *mut libc::termios)
        }
        libc::TIOCGLCKTRMIOS => return cmd_unimplemented("TIOCGLCKTRMIOS"),
        libc::TIOCSLCKTRMIOS => return cmd_unimplemented("TIOCSLCKTRMIOS"),
        libc::TCSBRK => return intercept::tcsendbreak_chan(chan, fd, arg as libc::c_int),
        libc::TCSBRKP => return intercept::tcsendbreak_chan(chan, fd, arg as libc::c_int),
        libc::TCXONC => return intercept::tcflow_chan(chan, fd, arg as libc::c_int),
        libc::TIOCINQ => return ioctl_get_int(chan, fd, IoctlCall::FIONREAD, arg), // same as libc::FIONREAD
        libc::TCFLSH => return intercept::tcflush_chan(chan, fd, arg as libc::c_int),
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
            intercept::tcgetwinsize_chan(chan, fd, arg as *mut libc::winsize)
        }
        _ if cmd == libc::TIOCSWINSZ as _ => {
            intercept::tcsetwinsize_chan(chan, fd, arg as *mut libc::winsize)
        }
        _ if cmd == libc::TIOCSBRK as _ => cmd_unimplemented("TIOCSBRK"),
        _ if cmd == libc::TIOCCBRK as _ => cmd_unimplemented("TIOCCBRK"),
        _ if cmd == libc::FIONREAD as _ => ioctl_get_int(chan, fd, IoctlCall::FIONREAD, arg),
        _ if cmd == libc::TIOCOUTQ as _ => ioctl_get_int(chan, fd, IoctlCall::TIOCOUTQ, arg),
        _ if cmd == libc::TIOCSTI as _ => cmd_unimplemented("TIOCSTI"),
        _ if cmd == libc::TIOCCONS as _ => cmd_unimplemented("TIOCCONS"),
        _ if cmd == libc::TIOCSCTTY as _ => cmd_unimplemented("TIOCSCTTY"),
        _ if cmd == libc::TIOCNOTTY as _ => cmd_unimplemented("TIOCNOTTY"),
        _ if cmd == libc::TIOCGPGRP as _ => unsafe {
            *(arg as *mut _) = intercept::tcgetpgrp_chan(chan, fd);
            0
        },
        _ if cmd == libc::TIOCSPGRP as _ => {
            intercept::tcsetpgrp_chan(chan, fd, unsafe { *(arg as *mut libc::pid_t) })
        }
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
        _ => unsafe {
            debug("falling back to native ioctl");
            libc::ioctl(fd, cmd, arg)
        },
    }
}

fn ioctl_get_int(
    mut chan: RemoteChannel,
    fd: libc::c_int,
    cmd: IoctlCall,
    arg: *mut libc::c_void,
) -> libc::c_int {
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Ioctl(cmd),
    };

    // send ioctl request to remote
    let res = match chan.send(Channel::PTY, req) {
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

    ret as _
}

fn ioctl_set_int(mut chan: RemoteChannel, fd: libc::c_int, cmd: IoctlCall) -> libc::c_int {
    let req = PtySlaveCall {
        fd: Fd(fd),
        typ: PtySlaveCallType::Ioctl(cmd),
    };

    // send ioctl request to remote
    let res = match chan.send(Channel::PTY, req) {
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
    -1
}

#[cfg(test)]
mod tests {
    use remote_pty_common::{
        channel::{mock::MockChannel, Channel},
        proto::{
            slave::{
                IoctlCall, IoctlResponse, IoctlValueResponse, ProcGroupResponse, PtySlaveCall,
                PtySlaveCallType, PtySlaveResponse, SetProcGroupCall,
            },
            Fd,
        },
    };

    use super::ioctl_chan;

    #[test]
    fn test_unimplemented() {
        let mock = MockChannel::assert_sends::<PtySlaveCall, PtySlaveResponse>(
            Channel::PTY,
            vec![],
            vec![],
        );

        let res = ioctl_chan(
            mock.chan.clone(),
            1,
            libc::TIOCSBRK as _,
            &mut 10 as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, -1);
        assert_eq!(errno::errno().0, libc::EINVAL);
    }

    #[test]
    fn test_non_terminal_ioctl() {
        let mock = MockChannel::assert_sends::<PtySlaveCall, PtySlaveResponse>(
            Channel::PTY,
            vec![],
            vec![],
        );

        let res = ioctl_chan(
            mock.chan.clone(),
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

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);

        let res = ioctl_chan(
            mock.chan.clone(),
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

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::c_int) as *mut libc::c_int;

        let res = ioctl_chan(
            mock.chan.clone(),
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

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::c_int) as *mut libc::c_int;

        let res = ioctl_chan(
            mock.chan.clone(),
            1,
            libc::FIONREAD,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
        assert_eq!(unsafe { *val }, 10 as libc::c_int);
    }

    #[test]
    fn test_ioctl_tiocgpgrp() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetProcGroup,
        };
        let mock_res = PtySlaveResponse::GetProcGroup(ProcGroupResponse { pid: 123 });

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::pid_t) as *mut libc::pid_t;

        let res = ioctl_chan(
            mock.chan.clone(),
            1,
            libc::TIOCGPGRP,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
        assert_eq!(unsafe { *val }, 123 as libc::c_int);
    }

    #[test]
    fn test_ioctl_tiocspgrp() {
        let expected_req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::SetProgGroup(SetProcGroupCall { pid: 123 }),
        };
        let mock_res = PtySlaveResponse::Success(0);

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);
        let val = &mut (123 as libc::pid_t) as *mut libc::pid_t;

        let res = ioctl_chan(
            mock.chan.clone(),
            1,
            libc::TIOCSPGRP,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
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

        let mock = MockChannel::assert_sends(Channel::PTY, vec![expected_req], vec![mock_res]);
        let val = &mut (0 as libc::c_int) as *mut libc::c_int;

        let res = ioctl_chan(
            mock.chan.clone(),
            1,
            libc::TIOCINQ,
            val as *mut _ as *mut libc::c_void,
        );

        assert_eq!(res, 0);
        assert_eq!(unsafe { *val }, 10 as libc::c_int);
    }
}
