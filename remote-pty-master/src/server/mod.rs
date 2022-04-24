pub mod acceptor;
pub mod listener;
pub mod pty;
pub mod signal;
pub mod stdin;

use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
};

use remote_pty_common::{
    channel::{Channel, RemoteChannel},
    log::debug,
    proto::{
        master::{PtyMasterCall, PtyMasterResponse, PtyMasterSignal, WriteStdinCall, SignalCall},
        slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, TcError, WriteStdoutCall},
    },
};

use crate::{context::Context, handler::RemotePtyHandlers};

use self::{
    acceptor::Acceptor, listener::Listener, pty::ClientPtyListener, signal::SignalWatcher,
    stdin::StdinReader,
};

pub struct Server {
    // server state
    ctx: Context,
    // listener
    listener: Option<Box<dyn Listener + Send>>,
    // list of clients indexed by pid
    clients: HashMap<u32, Client>,
    // event sender
    sender: Sender<Event>,
    // event receiver
    receiver: Receiver<Event>,
    // terminate flag
    terminate: Arc<AtomicBool>,
}

pub struct ServerHandle {
    //
    terminate: Arc<AtomicBool>,
    //
    sender: Sender<Event>,
    //
    handle: JoinHandle<()>,
}

#[derive(Clone)]
pub struct Client {
    chan: RemoteChannel,
    pid: u32,
    pgrp: u32,
}

pub enum Event {
    Stdin(Vec<u8>),
    Signal(PtyMasterSignal),
    ClientEvent(ClientEvent),
    Terminate,
}

pub struct ClientEvent {
    pub client: Client,
    pub event: ClientEventType,
}

pub enum ClientEventType {
    Registered,
    Call(PtySlaveCall),
    Terminated,
}

enum EventHandleResult {
    Success,
    ErrorIgnore,
    ErrorTerminateClient(Client),
    ErrorTerminateServer,
}

impl Server {
    pub fn new(ctx: Context, listener: Box<dyn Listener + Send>) -> Self {
        let (sender, receiver) = channel();

        Self {
            ctx,
            listener: Some(listener),
            clients: HashMap::new(),
            sender,
            receiver,
            terminate: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(self) -> ServerHandle {
        let terminate = Arc::clone(&self.terminate);
        let sender = self.sender.clone();
        let handle = thread::spawn(move || self.work());

        ServerHandle {
            terminate,
            sender,
            handle,
        }
    }

    fn work(mut self) {
        Acceptor::new(self.listener.take().unwrap(), &self.terminate, &self.sender).start();
        StdinReader::new(&self.terminate, &self.sender).start();
        SignalWatcher::new(&self.terminate, &self.sender).start();

        while !self.terminate.load(Ordering::Relaxed) {
            let evt = match self.receiver.recv() {
                Ok(evt) => evt,
                Err(err) => {
                    debug(format!("server could not recv event: {:?}", err));
                    return;
                }
            };

            let res = match evt {
                Event::Stdin(data) => self.handle_stdin(data),
                Event::Signal(sig) => self.handle_signal(sig),
                Event::ClientEvent(cevt) => self.handle_client_event(cevt),
                Event::Terminate => return,
            };

            self.handle_result(res);
        }
    }

    fn get_active_client(&self) -> Option<Client> {
        let state = self.ctx.state.lock().unwrap();

        let cur_pgrp = state.pgrp?;

        self.clients.get(&cur_pgrp).map(|i| i.clone())
    }

    fn handle_stdin(&self, data: Vec<u8>) -> EventHandleResult {
        let mut client = match self.get_active_client() {
            Some(c) => c,
            None => {
                debug("attempted write to stdin while no active pgrp, discarding");
                return EventHandleResult::ErrorIgnore;
            }
        };

        let res = client.chan.send::<PtyMasterCall, PtyMasterResponse>(
            Channel::STDIN,
            PtyMasterCall::WriteStdin(WriteStdinCall { data }),
        );

        match res {
            Ok(PtyMasterResponse::WriteSuccess) => EventHandleResult::Success,
            Ok(res) => self.unexpected_result(client, res),
            Err(err) => self.client_error(client, err),
        }
    }

    fn handle_signal(&self, signal: PtyMasterSignal) -> EventHandleResult {
        let mut client = match self.get_active_client() {
            Some(c) => c,
            None => {
                debug("attempted send signal while no active pgrp, discarding");
                return EventHandleResult::ErrorIgnore;
            }
        };

        let pgrp = {
            let state = self.ctx.state.lock().unwrap();
            state.pgrp.unwrap_or(client.pgrp)
        };

        let res = client
            .chan
            .send::<PtyMasterCall, PtyMasterResponse>(Channel::SIGNAL, PtyMasterCall::Signal(SignalCall {
                signal,
                pgrp
            }));

        match res {
            Ok(PtyMasterResponse::Success(_)) => EventHandleResult::Success,
            Ok(res) => self.unexpected_result(client, res),
            Err(err) => self.client_error(client, err),
        }
    }

    fn handle_client_event(&mut self, cevt: ClientEvent) -> EventHandleResult {
        let client = cevt.client;

        match cevt.event {
            ClientEventType::Registered => {
                self.register_client(client);
                EventHandleResult::Success
            }
            ClientEventType::Terminated => {
                self.remove_client(client.pid);
                EventHandleResult::Success
            }
            ClientEventType::Call(req) => self.handle_pty_call(client, req),
        }
    }

    fn handle_pty_call(&self, mut client: Client, req: PtySlaveCall) -> EventHandleResult {
        let active_client = self.get_active_client();

        // send signal to naughty procs
        if req.typ.must_be_foreground()
            && active_client.is_some()
            && active_client.unwrap().pgrp != client.pgrp
        {
            debug(format!(
                "received invalid request from background pgrp {}: {:?}",
                client.pgrp, req
            ));

            let _ = client.chan.send::<PtyMasterCall, PtyMasterResponse>(
                Channel::SIGNAL,
                PtyMasterCall::Signal(SignalCall {
                    signal: PtyMasterSignal::SIGTTOU,
                    pgrp: client.pgrp
                }),
            );
            let _ = client
                .chan
                .send_response(Channel::PTY, PtySlaveResponse::Error(TcError::EIO));

            return EventHandleResult::ErrorIgnore;
        }

        let (channel, res) = if let PtySlaveCallType::WriteStdout(req) = req.typ {
            (Channel::STDOUT, self.handle_stdout(req))
        } else {
            (Channel::PTY, Ok(RemotePtyHandlers::handle(&self.ctx, req)))
        };

        let res = match res {
            Ok(r) => r,
            Err(err) => {
                debug(err);
                return EventHandleResult::ErrorTerminateServer;
            }
        };

        let res = client.chan.send_response(channel, res);

        match res {
            Ok(_) => EventHandleResult::Success,
            Err(err) => self.client_error(client, err),
        }
    }

    fn handle_stdout(&self, req: WriteStdoutCall) -> Result<PtySlaveResponse, String> {
        io::stdout()
            .write_all(req.data.as_slice())
            .map_err(|e| format!("failed to write to stdout: {}", e))?;
        io::stdout()
            .flush()
            .map_err(|e| format!("failed to flush stdout: {}", e))?;

        Ok(PtySlaveResponse::Success(0))
    }

    fn unexpected_result(&self, client: Client, res: PtyMasterResponse) -> EventHandleResult {
        debug(format!(
            "received unexpected response from client {}: {:?}",
            client.pid, res
        ));
        EventHandleResult::ErrorTerminateClient(client)
    }

    fn client_error(&self, client: Client, err: String) -> EventHandleResult {
        debug(format!(
            "error while reading from channel for client {}: {}",
            client.pid, err
        ));
        EventHandleResult::ErrorTerminateClient(client)
    }

    fn handle_result(&mut self, res: EventHandleResult) {
        match res {
            EventHandleResult::Success => {}
            EventHandleResult::ErrorIgnore => {}
            EventHandleResult::ErrorTerminateClient(client) => {
                self.remove_client(client.pid);
            }
            EventHandleResult::ErrorTerminateServer => {
                debug("terminating server");
                self.terminate.store(true, Ordering::Relaxed);
            }
        }
    }

    fn remove_client(&mut self, pid: u32) {
        debug(format!("terminated process {}", pid));
        // TODO: signal clean up to pty listener when client terminated

        let _ = self.clients.remove(&pid);

        // relinquish the foreground process slot if all terminated
        let mut ctx = self.ctx.state.lock().unwrap();
        if let Some(pgrp) = ctx.pgrp {
            if !self.clients.iter().any(|i| i.1.pgrp == pgrp) {
                ctx.pgrp = None;
            }
        }
    }

    fn register_client(&mut self, client: Client) {
        debug(format!("registered process {}", client.pid));

        // if no foreground group adopt the first registered proc
        {
            let mut ctx = self.ctx.state.lock().unwrap();
            if ctx.pgrp.is_none() {
                let _ = ctx.pgrp.insert(client.pgrp);
            }
        }

        ClientPtyListener::new(&client, Channel::PTY, &self.terminate, &self.sender).start();
        ClientPtyListener::new(&client, Channel::STDOUT, &self.terminate, &self.sender).start();
        let _ = self.clients.insert(client.pid, client);
    }
}

impl ServerHandle {
    pub fn join(self) -> Result<(), String> {
        self.handle
            .join()
            .map_err(|err| format!("failed to join worker thread: {:?}", err))?;

        Ok(())
    }

    pub fn terminate(self) -> Result<(), String> {
        self.terminate.store(true, Ordering::Relaxed);
        self.sender
            .send(Event::Terminate)
            .map_err(|err| format!("failed to send terminate event: {}", err))?;

        self.join()
    }
}
