use std::{
    borrow::BorrowMut,
    env,
    num::ParseIntError,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;
use remote_pty_common::log::debug;

use crate::fd::get_inode_from_fd;

pub struct Conf {
    // the path of the unix socket to send pty requests
    pub sock_path: String,
    // stdin fd
    pub stdin_fd: i32,
    // stdout fds
    pub stdout_fds: Vec<i32>,
    // mutable state
    pub state: Mutex<State>,
}

pub struct State {
    // we store the inode numbers of the stdio fd's
    // so we can determine if fd's reference the same pipe after duping

    // stdin inode
    pub stdin_inode: Option<u64>,
    // stdout inode's
    pub stdout_inode: Option<u64>,
    // main thread id
    pub thread_id: i64,
}

impl Conf {
    fn from_env() -> Result<Self, &'static str> {
        Ok(Self {
            sock_path: env::var("RPTY_SOCK_PATH")
                .map_err(|_| "could not find env var RPTY_SOCK_PATH")?,
            //
            stdin_fd: env::var("RPTY_STDIN")
                .unwrap_or_else(|_| "0".to_string())
                .parse::<i32>()
                .map_err(|_| "failed to number in RPTY_STDIN")?,
            //
            stdout_fds: env::var("RPTY_STDOUT")
                .unwrap_or_else(|_| "1,2".to_string())
                .split(',')
                .map(|i| i.parse::<i32>())
                .collect::<Result<Vec<i32>, ParseIntError>>()
                .map_err(|_| "failed to parse numbers in RPTY_STDOUT")?,
            //
            state: Mutex::new(State::new()),
        })
    }

    pub(crate) fn is_stdio_fd(&self, fd: i32) -> bool {
        self.stdin_fd == fd || self.stdout_fds.contains(&fd)
    }

    // checks if the supplied fd is referencing one of the pipes
    // replacing stdio
    pub(crate) fn is_pty_fd(&self, fd: i32) -> bool {
        let inode = match get_inode_from_fd(fd) {
            Ok(inode) => inode,
            Err(_) => return false,
        };

        let state = self.state.lock().unwrap();
        state.stdin_inode == Some(inode) || state.stdout_inode == Some(inode)
    }

    pub(crate) fn is_main_thread(&self) -> bool {
        let state = self.state.lock().unwrap();

        #[cfg(target_os = "linux")]
        return state.thread_id == unsafe { libc::gettid() } as _;

        #[cfg(not(target_os = "linux"))]
        return true;
    }

    pub(crate) fn update_state(&self, f: impl FnOnce(&mut State)) {
        let mut state = self.state.lock().unwrap();
        f(state.borrow_mut());
    }
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            stdin_inode: None,
            stdout_inode: None,
            //
            #[cfg(target_os = "linux")]
            thread_id: unsafe { libc::gettid() } as _,
            #[cfg(not(target_os = "linux"))]
            thread_id: 0, // not implemented
        }
    }
}

lazy_static! {
    static ref GLOBAL_CONF: Mutex<Option<Arc<Conf>>> = Mutex::new(Option::None);
}

pub fn get_conf() -> Result<Arc<Conf>, &'static str> {
    lazy_static::initialize(&GLOBAL_CONF);

    let mut conf = GLOBAL_CONF
        .lock()
        .map_err(|_| "failed to lock conf mutex")?;

    if conf.is_none() {
        let _ = conf.insert(Arc::new(Conf::from_env()?));
    }

    Ok(Arc::clone(conf.as_ref().unwrap()))
}

pub(crate) fn clear_conf() -> Result<(), String> {
    debug("clear config");

    let mut conf = GLOBAL_CONF
        .lock()
        .map_err(|_| "failed to lock conf mutex")?;

    let _ = conf.take();
    Ok(())

    // let conf = match get_conf() {
    //     Ok(c) => c,
    //     Err(err) => {
    //         debug(format!("atfork: failed to get conf {}", err));
    //         return;
    //     }
    // };

    // let thread_id = {
    //     let mut state = conf.state.lock().unwrap();
    //     state.thread_id = unsafe { libc::gettid() } as _;
    //     state.thread_id
    // };
    // debug(format!("conf updated thread id to {}", thread_id));
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::conf::{get_conf, GLOBAL_CONF};

    #[test]
    fn test_get_conf() {
        // should be none until init from first call to get_remote_channel
        assert!(GLOBAL_CONF.lock().unwrap().is_none());

        // mock env vars
        let sock_path = "/tmp/remote-pty.sock";
        env::set_var("RPTY_SOCK_PATH", sock_path);
        env::set_var("RPTY_STDIN", "0");
        env::set_var("RPTY_STDOUT", "1,2");

        let conf = get_conf().expect("could not construct conf");
        assert_eq!(conf.sock_path, sock_path);
        assert_eq!(conf.stdin_fd, 0);
        assert_eq!(conf.stdout_fds, vec![1, 2]);

        env::remove_var("RPTY_SOCK_PATH");
        env::remove_var("RPTY_STDIN");
        env::remove_var("RPTY_STDOUT");
    }
}
