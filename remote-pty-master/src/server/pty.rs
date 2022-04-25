use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
};

use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::slave::PtySlaveCall,
};

use super::{ClientEvent, ClientEventType, Event};

pub(crate) struct ClientPtyListener {
    client_pid: u32,
    chan: RemoteChannel,
    chan_type: Channel,
    terminate: Arc<AtomicBool>,
    sender: Sender<Event>,
}

impl ClientPtyListener {
    pub(crate) fn new(
        client_pid: u32,
        chan: RemoteChannel,
        chan_type: Channel,
        terminate: &Arc<AtomicBool>,
        sender: &Sender<Event>,
    ) -> Self {
        Self {
            client_pid,
            chan,
            chan_type,
            terminate: Arc::clone(terminate),
            sender: sender.clone(),
        }
    }

    pub(crate) fn start(self) {
        thread::spawn(move || self.work());
    }

    fn work(mut self) {
        while !self.terminate.load(Ordering::Relaxed) {
            let req = self.chan.receive_request::<PtySlaveCall>(self.chan_type);

            let req = match req {
                Ok(r) => r,
                Err(err) => {
                    debug(format!(
                        "error while receiving pty message from client (pid: {}): {}",
                        self.client_pid, err
                    ));
                    break;
                }
            };

            let res = self.sender.send(Event::ClientEvent(ClientEvent {
                client_pid: self.client_pid,
                event: ClientEventType::Call(req),
            }));

            match res {
                Ok(_) => {}
                Err(err) => {
                    debug(format!("failed to send pty event: {}", err));
                    break;
                }
            }
        }

        let res = self.sender.send(Event::ClientEvent(ClientEvent {
            client_pid: self.client_pid,
            event: ClientEventType::Terminated,
        }));
        debug(format!("terminating client {:?} listener: {:?}", self.chan_type, res));
    }
}
