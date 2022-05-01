use std::{
    collections::HashMap,
    hash::Hash,
    mem::MaybeUninit,
    ops::{BitAnd, BitOr},
};

use bincode::{Decode, Encode};

// Derived from libc termios structure
// @see https://code.woboq.org/userspace/glibc/sysdeps/unix/sysv/linux/bits/termios-struct.h.html#termios
// @see https://fossies.org/dox/musl-1.2.2/structtermios.html
// @see https://www.man7.org/linux/man-pages/man3/termios.3.html
// TODO: for better cross platform support we should parse & decompose termios
// bit flags from the slave into enums that can be reconstructed on the master
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Termios {
    pub c_iflag: Vec<TermiosInputMode>,
    pub c_oflag: Vec<TermiosOutputMode>,
    pub c_cflag: Vec<TermiosControlMode>,
    pub c_lflag: Vec<TermiosLocalMode>,
    pub c_cc: HashMap<TermiosControlChar, u8>,
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum TermiosInputMode {
    IGNBRK = 1,
    BRKINT = 2,
    IGNPAR = 3,
    PARMRK = 4,
    INPCK = 5,
    ISTRIP = 6,
    INLCR = 7,
    IGNCR = 8,
    ICRNL = 9,
    IXON = 11,
    IXANY = 12,
    IXOFF = 13,
    IMAXBEL = 14,
    IUTF8 = 15,
}

lazy_static::lazy_static! {
    static ref TERMIOS_INPUT_MODES: HashMap<TermiosInputMode, libc::tcflag_t> = [
        (TermiosInputMode::IGNBRK, libc::IGNBRK),
        (TermiosInputMode::BRKINT, libc::BRKINT),
        (TermiosInputMode::IGNPAR, libc::IGNPAR),
        (TermiosInputMode::PARMRK, libc::PARMRK),
        (TermiosInputMode::INPCK, libc::INPCK),
        (TermiosInputMode::ISTRIP, libc::ISTRIP),
        (TermiosInputMode::INLCR, libc::INLCR),
        (TermiosInputMode::IGNCR, libc::IGNCR),
        (TermiosInputMode::ICRNL, libc::ICRNL),
        (TermiosInputMode::IXON, libc::IXON),
        (TermiosInputMode::IXANY, libc::IXANY),
        (TermiosInputMode::IXOFF, libc::IXOFF),
        (TermiosInputMode::IMAXBEL, libc::IMAXBEL),
        (TermiosInputMode::IUTF8, libc::IUTF8),
    ].iter().copied().collect();
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum TermiosOutputMode {
    OPOST = 1,
    OLCUC = 2,
    ONLCR = 3,
    OCRNL = 4,
    ONOCR = 5,
    ONLRET = 6,
    OFILL = 7,
    OFDEL = 8,
    CR0 = 15,
    CR1 = 16,
    CR2 = 17,
    CR3 = 18,
    TAB0 = 19,
    TAB1 = 20,
    TAB2 = 21,
    TAB3 = 22,
    VT0 = 23,
    VT1 = 24,
    FF0 = 25,
    FF1 = 26,
    BS0 = 27,
    BS1 = 28,
    NL0 = 29,
    NL1 = 30,
}

lazy_static::lazy_static! {
    static ref TERMIOS_OUTPUT_MODES: HashMap<TermiosOutputMode, libc::tcflag_t> = [
        (TermiosOutputMode::OPOST, libc::OPOST),
        #[cfg(target_os = "linux")]
        (TermiosOutputMode::OLCUC, libc::OLCUC),
        (TermiosOutputMode::ONLCR, libc::ONLCR),
        (TermiosOutputMode::OCRNL, libc::OCRNL),
        (TermiosOutputMode::ONOCR, libc::ONOCR),
        (TermiosOutputMode::ONLRET, libc::ONLRET),
        (TermiosOutputMode::OFILL, libc::OFILL),
        (TermiosOutputMode::OFDEL, libc::OFDEL),
        (TermiosOutputMode::NL0, libc::NL0),
        (TermiosOutputMode::NL1, libc::NL1),
        (TermiosOutputMode::CR0, libc::CR0 as _),
        (TermiosOutputMode::CR1, libc::CR1 as _),
        (TermiosOutputMode::CR2, libc::CR2 as _),
        (TermiosOutputMode::CR3, libc::CR3 as _),
        (TermiosOutputMode::TAB0, libc::TAB0 as _),
        (TermiosOutputMode::TAB1, libc::TAB1 as _),
        (TermiosOutputMode::TAB2, libc::TAB2 as _),
        (TermiosOutputMode::TAB3, libc::TAB3 as _),
        (TermiosOutputMode::BS0, libc::BS0 as _),
        (TermiosOutputMode::BS1, libc::BS1 as _),
        (TermiosOutputMode::VT0, libc::VT0 as _),
        (TermiosOutputMode::VT1, libc::VT1 as _),
        (TermiosOutputMode::FF0, libc::FF0 as _),
        (TermiosOutputMode::FF1, libc::FF1 as _),
    ].iter().copied().collect();
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum TermiosControlMode {
    CSTOPB = 4,
    CREAD = 5,
    PARENB = 6,
    PARODD = 7,
    HUPCL = 8,
    CLOCAL = 9,
    // CIBAUD = 11,
    CMSPAR = 12,
    CRTSCTS = 13,
    CS5 = 14,
    CS6 = 15,
    CS7 = 16,
    CS8 = 17,
    B0 = 18,
    B50 = 19,
    B75 = 20,
    B110 = 21,
    B134 = 22,
    B150 = 23,
    B200 = 24,
    B300 = 25,
    B600 = 26,
    B1200 = 27,
    B1800 = 28,
    B2400 = 29,
    B4800 = 30,
    B9600 = 31,
    B19200 = 32,
    B38400 = 33,
    EXTA = 34,
    EXTB = 35,
    B57600 = 36,
    B115200 = 37,
    B230400 = 38,
    B460800 = 39,
    B500000 = 40,
    B576000 = 41,
    B921600 = 42,
    B1000000 = 43,
    B1152000 = 44,
    B1500000 = 45,
    B2000000 = 46,
    B2500000 = 47,
    B3000000 = 48,
    B3500000 = 49,
    B4000000 = 50,
}

lazy_static::lazy_static! {
    static ref TERMIOS_CONTROL_MODES: HashMap<TermiosControlMode, libc::tcflag_t> = [
        (TermiosControlMode::CS5, libc::CS5),
        (TermiosControlMode::CS6, libc::CS6),
        (TermiosControlMode::CS7, libc::CS7),
        (TermiosControlMode::CS8, libc::CS8),
        (TermiosControlMode::CSTOPB, libc::CSTOPB),
        (TermiosControlMode::CREAD, libc::CREAD),
        (TermiosControlMode::PARENB, libc::PARENB),
        (TermiosControlMode::PARODD, libc::PARODD),
        (TermiosControlMode::HUPCL, libc::HUPCL),
        (TermiosControlMode::CLOCAL, libc::CLOCAL),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::CMSPAR, libc::CMSPAR),
        (TermiosControlMode::CRTSCTS, libc::CRTSCTS),
        (TermiosControlMode::B0, libc::B0),
        (TermiosControlMode::B50, libc::B50),
        (TermiosControlMode::B75, libc::B75),
        (TermiosControlMode::B110, libc::B110),
        (TermiosControlMode::B134, libc::B134),
        (TermiosControlMode::B150, libc::B150),
        (TermiosControlMode::B200, libc::B200),
        (TermiosControlMode::B300, libc::B300),
        (TermiosControlMode::B600, libc::B600),
        (TermiosControlMode::B1200, libc::B1200),
        (TermiosControlMode::B1800, libc::B1800),
        (TermiosControlMode::B2400, libc::B2400),
        (TermiosControlMode::B4800, libc::B4800),
        (TermiosControlMode::B9600, libc::B9600),
        (TermiosControlMode::B19200, libc::B19200),
        (TermiosControlMode::B38400, libc::B38400),
        (TermiosControlMode::EXTA, libc::EXTA),
        (TermiosControlMode::EXTB, libc::EXTB),
        (TermiosControlMode::B57600, libc::B57600),
        (TermiosControlMode::B115200, libc::B115200),
        (TermiosControlMode::B230400, libc::B230400),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B460800, libc::B460800),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B500000, libc::B500000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B576000, libc::B576000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B921600, libc::B921600),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B1000000, libc::B1000000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B1152000, libc::B1152000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B1500000, libc::B1500000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B2000000, libc::B2000000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B2500000, libc::B2500000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B3000000, libc::B3000000),
        #[cfg(target_os = "linux")]
        (TermiosControlMode::B3500000, libc::B3500000),
    ].iter().copied().collect();
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Copy, Hash)]
#[repr(u8)]
pub enum TermiosLocalMode {
    ISIG = 1,
    ICANON = 2,
    ECHO = 4,
    ECHOE = 5,
    ECHOK = 6,
    ECHONL = 7,
    ECHOCTL = 8,
    ECHOPRT = 9,
    ECHOKE = 10,
    FLUSHO = 12,
    NOFLSH = 13,
    TOSTOP = 14,
    PENDIN = 15,
    IEXTEN = 16,
}

lazy_static::lazy_static! {
    static ref TERMIOS_LOCAL_MODES: HashMap<TermiosLocalMode, libc::tcflag_t> = [
        (TermiosLocalMode::ISIG, libc::ISIG),
        (TermiosLocalMode::ICANON, libc::ICANON),
        (TermiosLocalMode::ECHO, libc::ECHO),
        (TermiosLocalMode::ECHOE, libc::ECHOE),
        (TermiosLocalMode::ECHOK, libc::ECHOK),
        (TermiosLocalMode::ECHONL, libc::ECHONL),
        (TermiosLocalMode::ECHOCTL, libc::ECHOCTL),
        (TermiosLocalMode::ECHOPRT, libc::ECHOPRT),
        (TermiosLocalMode::ECHOKE, libc::ECHOKE),
        (TermiosLocalMode::FLUSHO, libc::FLUSHO),
        (TermiosLocalMode::NOFLSH, libc::NOFLSH),
        (TermiosLocalMode::TOSTOP, libc::TOSTOP),
        (TermiosLocalMode::PENDIN, libc::PENDIN),
        (TermiosLocalMode::IEXTEN, libc::IEXTEN),
    ].iter().copied().collect();
}

#[derive(Encode, Decode, Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[repr(u8)]
pub enum TermiosControlChar {
    VDISCARD = 1,
    VDSUSP = 2,
    VEOF = 3,
    VERASE = 6,
    VINTR = 7,
    VKILL = 8,
    VLNEXT = 9,
    VMIN = 10,
    VQUIT = 11,
    VREPRINT = 12,
    VSTART = 13,
    VSTATUS = 14,
    VSTOP = 15,
    VSUSP = 16,
    VTIME = 18,
    VWERASE = 19,
}

lazy_static::lazy_static! {
    static ref TERMIOS_CONTROL_CHARS: HashMap<TermiosControlChar, usize> = [
        (TermiosControlChar::VDISCARD, libc::VDISCARD),
        #[cfg(target_os = "macos")]
        (TermiosControlChar::VDSUSP, libc::VDSUSP),
        (TermiosControlChar::VEOF, libc::VEOF),
        (TermiosControlChar::VERASE, libc::VERASE),
        (TermiosControlChar::VINTR, libc::VINTR),
        (TermiosControlChar::VKILL, libc::VKILL),
        (TermiosControlChar::VLNEXT, libc::VLNEXT),
        (TermiosControlChar::VMIN, libc::VMIN),
        (TermiosControlChar::VQUIT, libc::VQUIT),
        (TermiosControlChar::VREPRINT, libc::VREPRINT),
        (TermiosControlChar::VSTART, libc::VSTART),
        (TermiosControlChar::VSTOP, libc::VSTOP),
        (TermiosControlChar::VSUSP, libc::VSUSP),
        (TermiosControlChar::VTIME, libc::VTIME),
        (TermiosControlChar::VWERASE, libc::VWERASE),
    ].iter().copied().collect();
}

impl Termios {
    pub fn zeroed_libc_termios() -> libc::termios {
        unsafe { MaybeUninit::<libc::termios>::zeroed().assume_init() }
    }

    pub fn from_libc_termios(term: &libc::termios) -> Self {
        Self {
            c_iflag: from_bitflags(&TERMIOS_INPUT_MODES, term.c_iflag),
            c_oflag: from_bitflags(&TERMIOS_OUTPUT_MODES, term.c_oflag),
            c_cflag: from_bitflags(&TERMIOS_CONTROL_MODES, term.c_cflag),
            c_lflag: from_bitflags(&TERMIOS_LOCAL_MODES, term.c_lflag),
            c_cc: TERMIOS_CONTROL_CHARS
                .iter()
                .map(|(c, i)| (*c, term.c_cc[*i]))
                .collect(),
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
        term.c_iflag = to_bitflags(&TERMIOS_INPUT_MODES, &self.c_iflag);
        term.c_oflag = to_bitflags(&TERMIOS_OUTPUT_MODES, &self.c_oflag);
        term.c_cflag = to_bitflags(&TERMIOS_CONTROL_MODES, &self.c_cflag);
        term.c_lflag = to_bitflags(&TERMIOS_LOCAL_MODES, &self.c_lflag);

        #[cfg(target_os = "macos")]
        term.c_cc.fill(255);

        for (c, idx) in TERMIOS_CONTROL_CHARS.iter() {
            if let Some(val) = self.c_cc.get(c) {
                term.c_cc[*idx] = *val;
            }
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

fn from_bitflags<T, I>(map: &HashMap<T, I>, val: I) -> Vec<T>
where
    T: Eq + Hash + Copy,
    I: Copy + BitAnd<I, Output = I> + Eq + Default,
{
    let mut flags = Vec::<T>::new();

    for (k, v) in map.iter() {
        if val & *v == *v {
            flags.push(*k);
        }
    }

    flags
}

fn to_bitflags<T, I>(map: &HashMap<T, I>, val: &Vec<T>) -> I
where
    T: Eq + Hash + Copy,
    I: Copy + BitOr<I, Output = I> + Eq + Default,
{
    val.iter()
        .map(|i| *map.get(i).unwrap())
        .fold(I::default(), |acc, i| acc | i)
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

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::*;

    #[test]
    fn zero_termios() {
        let zeroed_termios = Termios::zeroed_libc_termios();

        assert_eq!(zeroed_termios.c_iflag, 0);
        assert_eq!(zeroed_termios.c_oflag, 0);
        assert_eq!(zeroed_termios.c_cflag, 0);
        assert_eq!(zeroed_termios.c_lflag, 0);
        assert_eq!(*zeroed_termios.c_cc.iter().max().unwrap(), 0);
    }

    #[test]
    fn test_from_bitflags() {
        let flag = libc::IGNBRK | libc::BRKINT;

        let vec = from_bitflags(&TERMIOS_INPUT_MODES, flag);

        assert!(vec.contains(&TermiosInputMode::IGNBRK));
        assert!(vec.contains(&TermiosInputMode::BRKINT));
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn test_to_bitflags() {
        let vec = vec![TermiosInputMode::IGNBRK, TermiosInputMode::BRKINT];

        let flag = to_bitflags(&TERMIOS_INPUT_MODES, &vec);

        assert_eq!(flag, libc::IGNBRK | libc::BRKINT);
    }

    #[test]
    fn test_from_libc_termios_zeroed() {
        let zeroed_termios = Termios::zeroed_libc_termios();

        let termios = Termios::from_libc_termios(&zeroed_termios);

        assert_eq!(termios.c_iflag, vec![]);
        assert!(termios.c_oflag.contains(&TermiosOutputMode::TAB0));
        assert!(termios.c_oflag.contains(&TermiosOutputMode::CR0));
        assert!(termios.c_oflag.contains(&TermiosOutputMode::VT0));
        assert!(termios.c_oflag.contains(&TermiosOutputMode::FF0));
        assert!(termios.c_oflag.contains(&TermiosOutputMode::VT0));
        assert!(termios.c_oflag.contains(&TermiosOutputMode::BS0));
        assert_eq!(termios.c_oflag.len(), 6);
        assert!(termios.c_cflag.contains(&TermiosControlMode::B0));
        assert!(termios.c_cflag.contains(&TermiosControlMode::CS5));
        assert_eq!(termios.c_cflag.len(), 2);
        assert_eq!(termios.c_lflag, vec![]);
        assert_eq!(*termios.c_cc.values().max().unwrap(), 0);
    }

    #[test]
    fn test_to_and_from_zero_termios() {
        let zeroed_termios = Termios::zeroed_libc_termios();

        let termios = Termios::from_libc_termios(&zeroed_termios);

        let mut written_termios = Termios::zeroed_libc_termios();
        termios.copy_to_libc_termios(&mut written_termios);

        assert_eq!(written_termios.c_iflag, 0);
        assert_eq!(written_termios.c_oflag, 0);
        assert_eq!(written_termios.c_cflag, 0);
        assert_eq!(written_termios.c_lflag, 0);
        #[cfg(target_os = "macos")]
        assert_eq!(*written_termios.c_cc.iter().max().unwrap(), 255);
        #[cfg(target_os = "linux")]
        assert_eq!(*written_termios.c_cc.iter().max().unwrap(), 0);
    }

    #[test]
    fn test_to_and_from_real_termios() {
        let (master, _) = unsafe {
            let mut master = -1 as libc::c_int;
            let mut slave = -1 as libc::c_int;
            let nullptr = ptr::null_mut::<u8>();

            let ret = libc::openpty(
                &mut master,
                &mut slave,
                nullptr as *mut _,
                nullptr as *mut _,
                nullptr as *mut _,
            );

            assert!(ret == 0);

            (master, slave)
        };

        let real_termios = unsafe {
            let mut termios = Termios::zeroed_libc_termios();
            let ret = libc::tcgetattr(master, &mut termios as *mut _);
            assert!(ret == 0);
            termios
        };

        let mut written_termios = Termios::zeroed_libc_termios();
        Termios::from_libc_termios(&real_termios).copy_to_libc_termios(&mut written_termios);

        dbg!(real_termios.c_cflag);
        dbg!(from_bitflags(
            &crate::proto::structs::TERMIOS_CONTROL_MODES,
            real_termios.c_cflag
        ));
        assert_eq!(written_termios.c_iflag, real_termios.c_iflag);
        assert_eq!(written_termios.c_oflag, real_termios.c_oflag);
        assert_eq!(written_termios.c_cflag, real_termios.c_cflag);
        assert_eq!(written_termios.c_lflag, real_termios.c_lflag);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(written_termios.c_cc, real_termios.c_cc);
        #[cfg(any(target_env = "gnu", target_os = "macos"))]
        {
            assert_eq!(written_termios.c_ispeed, real_termios.c_ispeed);
            assert_eq!(written_termios.c_ospeed, real_termios.c_ospeed);
        }
    }
}
