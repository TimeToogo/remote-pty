use std::{sync::Arc, fmt::Debug};


use remote_pty_common::log::debug;

use crate::{channel::{RemoteChannel, get_remote_channel}, conf::get_conf};

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
    F1: FnOnce(Arc<dyn RemoteChannel>) -> R,
    F2: FnOnce() -> R,
    S: Into<String>
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
    if !conf.fds.contains(&(fd as _)) {
        debug("falling back to libc implementation as fd is not configured");
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
