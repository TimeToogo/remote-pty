use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread,
};

use remote_pty_common::{channel::Channel, log::debug, proto::slave::PtySlaveCall};

use super::{Client, ClientEvent, ClientEventType, Event};

pub(crate) struct ClientPtyListener {
    client: Client,
    channel: Channel,
    terminate: Arc<AtomicBool>,
    sender: Sender<Event>,
}

impl ClientPtyListener {
    pub(crate) fn new(
        client: &Client,
        channel: Channel,
        terminate: &Arc<AtomicBool>,
        sender: &Sender<Event>,
    ) -> Self {
        Self {
            client: client.clone(),
            channel,
            terminate: Arc::clone(terminate),
            sender: sender.clone(),
        }
    }

    pub(crate) fn start(self) {
        thread::spawn(move || self.work());
    }

    fn work(mut self) {
        while !self.terminate.load(Ordering::Relaxed) {
            let req = self
                .client
                .chan
                .receive_request::<PtySlaveCall>(self.channel);

            let req = match req {
                Ok(r) => r,
                Err(err) => {
                    debug(format!(
                        "error while receiving pty message from client: {}",
                        err
                    ));
                    break;
                }
            };

            let res = self.sender.send(Event::ClientEvent(ClientEvent {
                client: self.client.clone(),
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

        let _ = self.sender.send(Event::ClientEvent(ClientEvent {
            client: self.client.clone(),
            event: ClientEventType::Terminated,
        }));
        debug(format!("terminating client {:?} listener", self.channel));
    }
}
