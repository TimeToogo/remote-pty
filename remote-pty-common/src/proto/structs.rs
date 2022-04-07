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

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Fd(pub i32);