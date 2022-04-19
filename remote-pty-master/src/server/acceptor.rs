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
    proto::slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcError},
};

use super::{listener::Listener, Client, ClientEvent, ClientEventType, Event};

pub(crate) struct Acceptor {
    listener: Box<dyn Listener + Send>,
    terminate: Arc<AtomicBool>,
    sender: Sender<Event>,
}

impl Acceptor {
    pub(crate) fn new(
        listener: Box<dyn Listener + Send>,
        terminate: &Arc<AtomicBool>,
        sender: &Sender<Event>,
    ) -> Self {
        Self {
            listener,
            terminate: Arc::clone(terminate),
            sender: sender.clone(),
        }
    }

    pub(crate) fn start(self) {
        thread::spawn(move || self.work());
    }

    fn work(mut self) {
        while !self.terminate.load(Ordering::Relaxed) {
            let res = self.listener.accept();

            let chan = match res {
                Ok(c) => c,
                Err(err) => {
                    debug(format!("error while accepting connection: {}", err));
                    return;
                }
            };

            debug("received connection");
            self.handle_connection(chan);
        }

        debug("terminating acceptor");
    }

    fn handle_connection(&self, mut chan: RemoteChannel) {
        let sender = self.sender.clone();

        thread::spawn(move || {
            let chan_send = chan.clone();

            let res = chan.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PGRP, |req| {
                let req = match req {
                    PtySlaveCall {
                        fd: _,
                        typ: PtySlaveCallType::RegisterProcess(req),
                    } => req,
                    req => {
                        debug(format!(
                            "unexpected request while accepting connection: {:?}",
                            req
                        ));
                        return PtySlaveResponse::Error(TcError::EIO);
                    }
                };

                let res = sender.send(Event::ClientEvent(ClientEvent {
                    client: Client {
                        chan: chan_send,
                        pgrp: req.pgrp,
                        pid: req.pid,
                    },
                    event: ClientEventType::Registered,
                }));

                match res {
                    Ok(_) => PtySlaveResponse::Success(0),
                    Err(err) => {
                        debug(format!("failed send registered event: {}", err));
                        PtySlaveResponse::Error(TcError::EIO)
                    }
                }
            });

            if let Err(err) = res {
                debug(format!("failed to register new process: {}", err));
            }
        });
    }
}
