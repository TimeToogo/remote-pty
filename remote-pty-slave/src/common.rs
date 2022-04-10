use std::{sync::Arc, ffi::CString};


use remote_pty_common::log::debug;

use crate::{channel::{RemoteChannel, get_remote_channel}, conf::get_conf};

// boilerplate logic for intercepting a libc function
// operating on a fd
pub(crate) fn handle_intercept<R, F1, F2>(
    func_name: &'static str,
    fd: libc::c_int,
    remote_cb: F1,
    #[allow(unused_variables)]
    staticlib_fallback_cb: F2,
    // #[allow(unused_variables)]
    // dylib_fallback_cb: F3,
) -> R
where
    R: From<i32>,
    F1: FnOnce(Arc<dyn RemoteChannel>) -> R,
    F2: FnOnce() -> R,
    // F3: FnOnce() -> R,
{
    debug(format!("intercepted {}", func_name));

    #[cfg(not(feature = "dylib"))]
    let fallback_cb = staticlib_fallback_cb;
    #[cfg(feature = "dylib")]
    let fallback_cb = dylib_fallback_cb;

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

// fallback to "shadowed" symbols in the current proc
// this is used for the LD_PRELOAD version of the 
pub(crate) unsafe fn dylib_fallback<A1, R>(symbol_name: &'static str, arg1: A1) -> R {
    let next_sym = libc::dlsym(libc::RTLD_NEXT, CString::new(symbol_name).unwrap().as_ptr());
    let next_sym = next_sym as *const _ as *const extern "C" fn(A1) -> R;

    (*next_sym)(arg1)
}