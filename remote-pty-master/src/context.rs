pub struct Context {
    pub pty: PtyPair,
}

pub struct PtyPair {
    pub master: PtyFd,
    pub slave: PtyFd,
}

pub type PtyFd = libc::c_int;

#[cfg(test)]
impl Context {
    pub fn from_pair(master: PtyFd, slave: PtyFd) -> Self {
        return Self {
            pty: PtyPair { master, slave },
        };
    }

    pub fn valid_pty_pair() -> Self {
        use std::ptr;

        let (master, slave) = unsafe {
            let mut master = -1 as libc::c_int;
            let mut slave = -1 as libc::c_int;
            let nullptr = ptr::null_mut::<u8>();

            // @see https://man7.org/linux/man-pages/man3/openpty.3.html
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

        Self::from_pair(master, slave)
    }

    pub fn not_pty_fds() -> Self {
        use std::ffi::CString;

        let fd = unsafe {
            let dn = CString::new("/dev/null").unwrap();
            libc::open(dn.as_ptr(), libc::O_RDONLY)
        };

        Self::from_pair(fd, fd)
    }

    pub fn invalid_fds() -> Self {
        Self::from_pair(-100, -100)
    }
}
