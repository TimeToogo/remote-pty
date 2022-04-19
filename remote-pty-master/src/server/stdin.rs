use std::{
    io::{self, Read},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
};

use remote_pty_common::log::debug;

use super::Event;

pub(crate) struct StdinReader {
    terminate: Arc<AtomicBool>,
    sender: Sender<Event>,
}

impl StdinReader {
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
        while !self.terminate.load(Ordering::Relaxed) {
            let mut buf = [0u8; 1024];

            loop {
                let res = io::stdin().read(&mut buf);

                let n = match res {
                    Ok(0) => {
                        debug("stdin eof detected, terminating server");
                        let _ = self.sender.send(Event::Terminate);
                        break;
                    }
                    Ok(n) => n,
                    Err(err) => {
                        debug(format!(
                            "failed to read from stdin {}, terminating server",
                            err
                        ));
                        let _ = self.sender.send(Event::Terminate);
                        break;
                    }
                };

                let res = self.sender.send(Event::Stdin(buf[..n].to_vec()));

                match res {
                    Ok(_) => continue,
                    Err(err) => {
                        debug(format!("failed to send stdin message: {}", err));
                        break;
                    }
                }
            }
        }

        debug("terminating stdin reader");
    }
}
