use std::sync::Arc;

use errno::{set_errno, Errno};
use remote_pty_common::log::debug;

use crate::{channel::RemoteChannel, common::handle_intercept};

#[cfg(target_os = "linux")]
type Cmd = libc::Ioctl;
#[cfg(not(target_os = "linux"))]
type Cmd = libc::c_ulong;

// @see https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
// @see https://pubs.opengroup.org/onlinepubs/007904975/functions/ioctl.html
// @see https://code.woboq.org/userspace/glibc/sysdeps/unix/sysv/linux/powerpc/ioctl.c.html
#[no_mangle]
pub extern "C" fn remote_ioctl(fd: libc::c_int, cmd: Cmd, arg: *mut libc::c_void) -> libc::c_int {
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
        libc::TCSETS => return crate::tcsetattr_chan(chan, fd, libc::TCSANOW, arg as *mut libc::termios),
        libc::TCSETSW => return crate::tcsetattr_chan(chan, fd, libc::TCSADRAIN, arg as *mut libc::termios),
        libc::TCSETSF => return crate::tcsetattr_chan(chan, fd, libc::TCSAFLUSH, arg as *mut libc::termios),
        libc::TIOCGLCKTRMIOS => return cmd_unimplemented("TIOCGLCKTRMIOS"),
        libc::TIOCSLCKTRMIOS => return cmd_unimplemented("TIOCSLCKTRMIOS"),
        libc::TCSBRK => return crate::tcsendbreak_chan(chan, fd, arg as libc::c_int),
        libc::TCSBRKP => return crate::tcsendbreak_chan(chan, fd, arg as libc::c_int),
        libc::TCXONC => return crate::tcflow_chan(chan, fd, arg as libc::c_int),
        libc::TIOCINQ => todo!(),
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
        libc::TIOCLINUX => return cmd_unimplemented("TIOCLINUX"),
        _ => {}
    };

    match cmd {
        _ if cmd == libc::TIOCGWINSZ as _ => crate::tcgetwinsize_chan(chan, fd, arg as *mut libc::winsize),
        _ if cmd == libc::TIOCSWINSZ as _ => crate::tcsetwinsize_chan(chan, fd, arg as *mut libc::winsize),
        _ if cmd == libc::TIOCSBRK as _ => cmd_unimplemented("TIOCSBRK"),
        _ if cmd == libc::TIOCCBRK as _ => cmd_unimplemented("TIOCCBRK"),
        _ if cmd == libc::FIONREAD as _ => todo!(),
        _ if cmd == libc::TIOCOUTQ as _ => todo!(),
        _ if cmd == libc::TIOCSTI as _ => cmd_unimplemented("TIOCSTI"),
        _ if cmd == libc::TIOCCONS as _ => cmd_unimplemented("TIOCCONS"),
        _ if cmd == libc::TIOCSCTTY as _ => cmd_unimplemented("TIOCSCTTY"),
        _ if cmd == libc::TIOCNOTTY as _ => cmd_unimplemented("TIOCNOTTY"),
        _ if cmd == libc::TIOCGPGRP as _ => cmd_unimplemented("TIOCGPGRP"),
        _ if cmd == libc::TIOCSPGRP as _ => cmd_unimplemented("TIOCSPGRP"),
        _ if cmd == libc::TIOCEXCL as _ => cmd_unimplemented("TIOCEXCL"),
        _ if cmd == libc::TIOCNXCL as _ => cmd_unimplemented("TIOCNXCL"),
        _ if cmd == libc::TIOCGETD as _ => todo!(),
        _ if cmd == libc::TIOCSETD as _ => todo!(),
        _ if cmd == libc::TIOCPKT as _ => cmd_unimplemented("TIOCPKT"),
        _ if cmd == libc::TIOCMGET as _ => cmd_unimplemented("TIOCMGET"),
        _ if cmd == libc::TIOCMSET as _ => cmd_unimplemented("TIOCMSET"),
        _ if cmd == libc::TIOCMBIC as _ => cmd_unimplemented("TIOCMBIC"),
        _ if cmd == libc::TIOCMBIS as _ => cmd_unimplemented("TIOCMBIS"),
        _ => unsafe { libc::ioctl(fd, cmd, arg) },
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
        slave::{ PtySlaveCall, PtySlaveResponse},
        Fd,
    };

    use crate::{channel::mock::MockChannel, ioctl::{self}};


    #[test]
    fn test_ioctl() {
        println!("test ioctl: {}", crate::remote_ioctl as *mut u8 as u64);
        println!("test libc::ioctl: {}", libc::ioctl as *mut u8 as u64);
        // println!("test libc_ioctl: {}", libc_ioctl as *mut u8 as u64);
    }
}
