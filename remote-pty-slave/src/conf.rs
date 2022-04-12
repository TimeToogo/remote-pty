use std::{
    sync::{Arc, Mutex}, env, num::ParseIntError,
};

use lazy_static::lazy_static;

pub struct Conf {
    // the path of the unix socket to send pty requests
    pub sock_path: String,
    // which fds will be intercepted and treated as remote pty's
    pub fds: Vec<u16>,
}

impl Conf {
    fn from_env() -> Result<Self, &'static str> {
        Ok(Self {
            sock_path: env::var("REMOTE_PTY_SOCK_PATH").map_err(|_| "could not find env var REMOTE_PTY_SOCK_PATH")?,
            fds: env::var("REMOTE_PTY_FDS").map_err(|_| "could not find env var REMOTE_PTY_FDS")?
                .split(',')
                .map(|i| i.parse::<u16>())
                .collect::<Result<Vec<u16>, ParseIntError>>()
                .map_err(|_| "failed to parse numbers in REMOTE_PTY_FDS")?,
        })
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

    use crate::{ conf::{GLOBAL_CONF, get_conf}};

    #[test]
    fn test_get_conf() {
        // should be none until init from first call to get_remote_channel
        assert!(GLOBAL_CONF.lock().unwrap().is_none());
        
        // mock env vars
        let sock_path = "/tmp/remote-pty.sock";
        env::set_var("REMOTE_PTY_SOCK_PATH", sock_path);
        let fds = "0,1,2";
        env::set_var("REMOTE_PTY_FDS", fds);
        
        let conf = get_conf().expect("could not construct conf");
        assert_eq!(conf.sock_path, sock_path);
        assert_eq!(conf.fds, vec![0,1,2]);

        env::remove_var("REMOTE_PTY_SOCK_PATH");
        env::remove_var("REMOTE_PTY_FDS");
    }
}