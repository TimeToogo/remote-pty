use remote_pty_common::log::debug;

use crate::{channel::get_remote_channel, conf::get_conf};

// initialisation function that executes on process startup
// here we forward signals from the remote master to the local process
#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
#[no_mangle]
pub static REMOTE_PTY_INIT_SIGNAL_HANDLER: extern "C" fn() = {
    #[cfg_attr(all(target_os = "linux", not(test)), link_section = ".text.startup")]
    #[no_mangle]
    pub extern "C" fn remote_pty_init_signal_handler() {
        debug("signal handler");

        let conf = match get_conf() {
            Ok(conf) => conf,
            Err(err) => {
                debug(format!("failed to init config: {}", err));
                return;
            }
        };

        let mut remote_channel = match get_remote_channel(&conf) {
            Ok(chan) => chan,
            Err(err) => {
                debug(format!("failed to get remote channel: {}", err));
                return;
            }
        };

        // TODO

        debug("init signal handler");
    }
    remote_pty_init_signal_handler
};
