use std::{
    env,
    num::ParseIntError,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;

pub struct Conf {
    // the path of the unix socket to send pty requests
    pub sock_path: String,
    // stdin fd
    pub stdin_fd: i32,
    // stdout fds
    pub stdout_fds: Vec<i32>,
}

impl Conf {
    fn from_env() -> Result<Self, &'static str> {
        Ok(Self {
            sock_path: env::var("RPTY_SOCK_PATH")
                .map_err(|_| "could not find env var RPTY_SOCK_PATH")?,
            //
            stdin_fd: env::var("RPTY_STDIN")
                .map_err(|_| "could not find env var RPTY_STDIN")?
                .parse::<i32>()
                .map_err(|_| "failed to number in RPTY_STDIN")?,
            //
            stdout_fds: env::var("RPTY_STDOUT")
                .map_err(|_| "could not find env var RPTY_STDOUT")?
                .split(',')
                .map(|i| i.parse::<i32>())
                .collect::<Result<Vec<i32>, ParseIntError>>()
                .map_err(|_| "failed to parse numbers in RPTY_STDOUT")?,
        })
    }

    pub(crate) fn is_pty_fd(&self, fd: i32) -> bool {
        self.stdin_fd == fd || self.stdout_fds.contains(&fd)
    }
}

lazy_static! {
    static ref GLOBAL_CONF: Mutex<Option<Arc<Conf>>> = Mutex::new(Option::None);
}

pub fn get_conf() -> Result<Arc<Conf>, &'static str> {
    let mut conf = GLOBAL_CONF
        .lock()
        .map_err(|_| "failed to lock conf mutex")?;

    if conf.is_none() {
        let _ = conf.insert(Arc::new(Conf::from_env()?));
    }

    Ok(Arc::clone(conf.as_ref().unwrap()))
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
