use remote_pty_common::log::debug;

use crate::{channel::close_remote_channel, conf::clear_conf, init::remote_pty_init};

// re-initialises the process
pub extern "C" fn fork_handler() {
    debug("process fork");

    if let Err(msg) = close_remote_channel() {
        debug(format!("failed to close remote channel: {}", msg));
        return;
    }

    let pre_fork_state = match clear_conf() {
        Ok(s) => s,
        Err(msg) => {
            debug(format!("failed to clear conf: {}", msg));
            return;
        }
    };

    remote_pty_init(pre_fork_state);

    debug("fork complete");
}
