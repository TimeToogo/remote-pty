use std::fmt::Debug;

use remote_pty_common::{channel::RemoteChannel, log::debug};

use crate::{channel::get_remote_channel, conf::get_conf};

// boilerplate logic for intercepting a libc function
// operating on a fd
pub(crate) fn handle_intercept<R, F1, F2, S>(
    func_name: S,
    fd: libc::c_int,
    remote_cb: F1,
    fallback_cb: F2,
) -> R
where
    R: From<i32> + Debug,
    F1: FnOnce(RemoteChannel) -> R,
    F2: FnOnce() -> R,
    S: Into<String>,
{
    debug(format!("intercepted {} (fd: {})", func_name.into(), fd));

    // first we get the config from the env
    let conf = match get_conf() {
        Ok(conf) => conf,
        Err(msg) => {
            debug(msg);
            return fallback_cb();
        }
    };

    // if the function was called with an fd outside of the
    // configured list we ignore it and delegate to the
    // original libc implementation
    if !conf.is_pty_fd(fd as _) {
        debug("falling back to libc implementation as fd is not configured");
        return fallback_cb();
    }

    // if the caller is not in the main thread we don't intercept
    // the calls as libraries as it will interfere with the channel
    // on the main thread, potentially creating/stealing messages causing 
    // deadlocks
    #[cfg(target_os = "linux")]
    if conf.thread_id != unsafe { libc::gettid() } as _ {
        debug("called on non-main thread, not intercepting");
        return fallback_cb();
    }

    // else we get the channel and send the request to the remote
    let chan = match get_remote_channel(&conf) {
        Ok(chan) => chan,
        Err(msg) => {
            debug(msg);
            return fallback_cb();
        }
    };

    let res = remote_cb(chan);
    debug(format!("response: {:?}", res));

    res
}
