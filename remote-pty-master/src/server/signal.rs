use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
};

use remote_pty_common::{log::debug, proto::master::PtyMasterSignal};
use signal_hook::iterator::Signals;

use super::Event;

pub(crate) struct SignalWatcher {
    terminate: Arc<AtomicBool>,
    sender: Sender<Event>,
}

impl SignalWatcher {
    pub(crate) fn new(terminate: &Arc<AtomicBool>, sender: &Sender<Event>) -> Self {
        Self {
            terminate: Arc::clone(terminate),
            sender: sender.clone(),
        }
    }

    pub(crate) fn start(self) {
        thread::spawn(move || self.work());
    }

    fn work(self) {
        let sigs = Signals::new(&[
            libc::SIGWINCH,
            libc::SIGINT,
            libc::SIGTERM,
            libc::SIGCONT,
            libc::SIGTTOU,
            libc::SIGTTIN,
        ]);

        let mut sigs = match sigs {
            Ok(s) => s,
            Err(err) => {
                debug(format!("failed to start signal watcher: {}", err));
                let _ = self.sender.send(Event::Terminate);
                return;
            }
        };

        while !self.terminate.load(Ordering::Relaxed) {
            for sig in sigs.wait() {
                let sig = match sig {
                    libc::SIGWINCH => PtyMasterSignal::SIGWINCH,
                    libc::SIGINT => PtyMasterSignal::SIGINT,
                    libc::SIGTERM => PtyMasterSignal::SIGTERM,
                    libc::SIGCONT => PtyMasterSignal::SIGCONT,
                    libc::SIGTTOU => PtyMasterSignal::SIGTTOU,
                    libc::SIGTTIN => PtyMasterSignal::SIGTTIN,
                    _ => {
                        debug(format!("unexpected signal: {}", sig));
                        continue;
                    }
                };

                debug(format!("received signal: {:?}", sig));

                let res = self.sender.send(Event::Signal(sig));

                match res {
                    Ok(_) => continue,
                    Err(err) => {
                        debug(format!("failed to send signal message: {}", err));
                        break;
                    }
                }
            }
        }

        debug("terminating signal watcher");
    }
}
