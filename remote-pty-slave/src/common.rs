use std::sync::Arc;


use remote_pty_common::log::debug;

use crate::{channel::{RemoteChannel, get_remote_channel}, conf::get_conf};

// boilerplate logic for intercepting a libc function
// operating on a fd
pub(crate) fn handle_intercept<R, F1, F2>(
    func_name: &'static str,
    fd: libc::c_int,
    remote_cb: F1,
    fallback_cb: F2,
) -> R
where
    R: From<i32>,
    F1: FnOnce(Arc<dyn RemoteChannel>) -> R,
    F2: FnOnce() -> R,
    // F3: FnOnce() -> R,
{
    debug(format!("intercepted {}", func_name));

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

    remote_cb(chan)
}
