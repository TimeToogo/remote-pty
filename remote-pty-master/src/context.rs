use std::{io::{self, Result}, sync::{Arc, Mutex}};

#[derive(Debug, Clone)]
pub struct Context {
    pub pty: PtyPair,
    pub state: Arc<Mutex<TerminalState>>
}

#[derive(Debug, PartialEq, Clone)]
pub struct TerminalState {
    pub pgrp: i32
}

#[derive(Debug, PartialEq, Clone)]
pub struct PtyPair {
    pub master: PtyFd,
    pub slave: PtyFd,
}

pub type PtyFd = libc::c_int;

impl Context {
    pub fn from_pair(master: PtyFd, slave: PtyFd) -> Self {
        Self {
            pty: PtyPair { master, slave },
            state: Arc::new(Mutex::new(TerminalState::new()))
        }
    }

    pub fn openpty() -> Result<Self> {
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

            if ret != 0 {
                return Err(io::Error::last_os_error());
            }

            (master, slave)
        };

        Ok(Self::from_pair(master, slave))
    }
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            pgrp: -1
        }
    }
}

#[cfg(test)]
impl Context {
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
