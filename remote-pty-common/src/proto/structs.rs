use bincode::{Decode, Encode};

// Derived from libc termios structure
// @see https://code.woboq.org/userspace/glibc/sysdeps/unix/sysv/linux/bits/termios-struct.h.html#termios
// @see https://fossies.org/dox/musl-1.2.2/structtermios.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Termios {
    pub c_iflag: u32,   /* input mode flags */
    pub c_oflag: u32,   /* output mode flags */
    pub c_cflag: u32,   /* control mode flags */
    pub c_lflag: u32,   /* local mode flags */
    pub c_line: u8,     /* line discipline */
    pub c_cc: [u8; 32], /* control characters */
    pub c_ispeed: u32,  /* input speed */
    pub c_ospeed: u32,  /* output speed */
}

impl Termios {
    pub fn zeroed_libc_termios() -> libc::termios {
        libc::termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_cc: [0; libc::NCCS],
            #[cfg(target_os = "linux")]
            c_line: 0,
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ispeed: 0,
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ospeed: 0,
            #[cfg(target_env = "musl")]
            __c_ispeed: 0,
            #[cfg(target_env = "musl")]
            __c_ospeed: 0,
        }
    }

    pub fn from_libc_termios(term: &libc::termios) -> Self {
        #[allow(unused_mut, unused_assignments)]
        let mut c_line = 0;
        #[cfg(target_os = "linux")]
        {
            c_line = (*term).c_line as _;
        }

        let mut c_cc = (*term).c_cc.to_vec();
        c_cc.resize(32, 0);

        Self {
            c_iflag: term.c_iflag as _,
            c_oflag: term.c_oflag as _,
            c_cflag: term.c_cflag as _,
            c_lflag: term.c_lflag as _,
            c_line,
            c_cc: c_cc.try_into().expect("invalid cc length"),
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ispeed: term.c_ispeed as _,
            #[cfg(any(target_env = "gnu", target_os = "macos"))]
            c_ospeed: term.c_ospeed as _,
            #[cfg(target_env = "musl")]
            c_ispeed: term.__c_ispeed as _,
            #[cfg(target_env = "musl")]
            c_ospeed: term.__c_ospeed as _,
        }
    }

    pub fn copy_to_libc_termios(&self, term: &mut libc::termios) {
        // map remote termios back to local termios
        // TODO: improve naive mapping which may not be invariant across libc's?
        term.c_iflag = self.c_iflag as libc::tcflag_t;
        term.c_oflag = self.c_oflag as libc::tcflag_t;
        term.c_cflag = self.c_cflag as libc::tcflag_t;
        term.c_lflag = self.c_lflag as libc::tcflag_t;
        term
            .c_cc
            .copy_from_slice(&self.c_cc[..libc::NCCS]);
        #[cfg(target_os = "linux")]
        {
            term.c_line = self.c_line as libc::cc_t;
        }
        #[cfg(any(target_env = "gnu", target_os = "macos"))]
        {
            term.c_ispeed = self.c_ispeed as libc::speed_t;
            term.c_ospeed = self.c_ospeed as libc::speed_t;
        }
        #[cfg(target_env = "musl")]
        {
            term.__c_ispeed = self.c_ispeed as libc::speed_t;
            term.__c_ospeed = self.c_ospeed as libc::speed_t;
        }
    }
}

// @see https://code.woboq.org/userspace/glibc/sysdeps/unix/sysv/linux/bits/ioctl-types.h.html#winsize
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Fd(pub i32);