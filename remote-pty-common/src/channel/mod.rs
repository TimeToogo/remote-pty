pub mod mock;
pub mod transport;

use std::{
    fmt::Debug,
    io,
    sync::{Arc, Condvar, Mutex, MutexGuard},
};

use bincode::{Decode, Encode};

use self::transport::Transport;

// thread-safe channel used to send and receive commands from the remote side
pub struct RemoteChannel {
    // underlying transport for
    reader: Arc<Mutex<dyn io::Read + Send>>,
    writer: Arc<Mutex<dyn io::Write + Send>>,
    // encoding conf
    conf: bincode::config::Configuration,
    // used to wait for new messages
    receiver: Arc<MessageReceiver>,
}

struct MessageReceiver {
    queue: Mutex<Vec<Message>>,
    condvar: Condvar,
}

#[derive(Encode, Decode, PartialEq, Clone, Copy)]
pub enum Channel {
    PTY,
    STDIN,
    STDOUT,
}

// wrapper struct used for encoding/decoding messages in a generic format
#[derive(Encode, Decode)]
struct Message {
    chan: Channel,
    mode: MessageMode,
    data: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Clone, Copy)]
enum MessageMode {
    Request,
    Response,
}

impl RemoteChannel {
    pub fn new(transport: impl Transport) -> Self {
        let (reader, writer) = transport.split();
        Self {
            reader: Arc::new(Mutex::from(reader)),
            writer: Arc::new(Mutex::from(writer)),
            conf: bincode::config::standard(),
            receiver: Arc::new(MessageReceiver {
                queue: Mutex::new(vec![]),
                condvar: Condvar::new(),
            }),
        }
    }

    // receives and responds to a command from the remote
    // note: this only supports one thread receiving messages per chan
    pub fn receive<Req, Res, F>(&mut self, chan: Channel, handler: F) -> Result<(), String>
    where
        Req: Encode + Decode + Debug,
        Res: Encode + Decode + Debug,
        F: FnOnce(Req) -> Res,
    {
        let req = self.read_msg(chan, MessageMode::Request)?;
        let res = handler(req);
        self.write_msg(chan, MessageMode::Response, res)?;

        Ok(())
    }

    // makes an synchronous RPC style call to the remote
    // note: this only supports one thread sending messages per chan
    pub fn send<Req, Res>(&mut self, chan: Channel, req: Req) -> Result<Res, String>
    where
        Req: Encode + Decode + Debug,
        Res: Encode + Decode + Debug,
    {
        self.write_msg(chan, MessageMode::Request, req)?;
        let res = self.read_msg(chan, MessageMode::Response)?;

        Ok(res)
    }

    // serialise and write the request to the underlying transport
    fn write_msg<Req>(&mut self, chan: Channel, mode: MessageMode, req: Req) -> Result<(), String>
    where
        Req: Encode,
    {
        let data = bincode::encode_to_vec(req, self.conf)
            .map_err(|e| format!("failed to encode req: {}", e))?;

        let msg = Message { chan, mode, data };
        let data = bincode::encode_to_vec(msg, self.conf)
            .map_err(|e| format!("failed to encode message: {}", e))?;

        let mut writer = self.writer.lock().unwrap();
        writer
            .write_all(data.as_slice())
            .map_err(|e| format!("failed to send req: {}", e))?;

        Ok(())
    }

    fn read_msg<Res>(&self, chan: Channel, mode: MessageMode) -> Result<Res, String>
    where
        Res: Decode,
    {
        loop {
            // ensure mutex is unlocked after checking if message is available
            {
                let queue = self.receiver.queue.lock().unwrap();
                if let Some(res) = self.find_matching_message(queue, chan, mode)? {
                    return Ok(res);
                }
            }

            {
                // ensure only single thread is trying to read the next message
                let lock = self.reader.try_lock();

                if lock.is_err() {
                    // if another thread got the lock we will wait until it notifies
                    // and then check the queued messages again as the received message
                    // could be for any thread!
                    let queue = self.receiver.queue.lock().unwrap();
                    let queue = self.receiver.condvar.wait(queue).unwrap();

                    if let Some(res) = self.find_matching_message(queue, chan, mode)? {
                        return Ok(res);
                    }

                    continue;
                }

                let reader = lock.unwrap();
                // TODO: fix lifetime hack
                let reader =
                    unsafe { std::mem::transmute::<_, MutexGuard<'static, dyn io::Read>>(reader) };
                let msg = bincode::decode_from_std_read(&mut LockedMutexReader(reader), self.conf)
                    .map_err(|e| format!("failed to decode err: {}", e))?;

                let mut queue = self.receiver.queue.lock().unwrap();
                queue.push(msg);

                // trigger all waiting threads to check if the new message is for them
                self.receiver.condvar.notify_all();
            }
        }
    }

    fn find_matching_message<Res>(
        &self,
        mut queue: MutexGuard<Vec<Message>>,
        chan: Channel,
        mode: MessageMode,
    ) -> Result<Option<Res>, String>
    where
        Res: Decode,
    {
        if let Some(idx) = queue.iter().position(|m| m.chan == chan && m.mode == mode) {
            let msg = queue.remove(idx);

            let (res, _) = bincode::decode_from_slice(msg.data.as_slice(), self.conf)
                .map_err(|e| format!("failed to decode request: {}", e))?;

            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

// not too sure why this is necessary but for some reason
// the Read trait is not implemented on the mutex guard of the inner Read
struct LockedMutexReader<'a>(MutexGuard<'a, dyn io::Read>);
impl<'a> io::Read for LockedMutexReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Clone for RemoteChannel {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            writer: self.writer.clone(),
            conf: self.conf,
            receiver: self.receiver.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::{
        channel::{transport::mem::MemoryTransport, Channel, RemoteChannel},
        proto::{
            master::{PtyMasterCall, PtyMasterResponse, PtyMasterSignal, WriteStdinCall},
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse, WriteStdoutCall},
            Fd,
        },
    };

    #[test]
    fn test_send_receive_msg() {
        let (t1, t2) = MemoryTransport::pair();

        let mut c1 = RemoteChannel::new(t1);
        let mut c2 = RemoteChannel::new(t2);

        let req = PtySlaveCall {
            fd: Fd(1),
            typ: PtySlaveCallType::GetAttr,
        };
        let req2 = req.clone();
        let res = PtySlaveResponse::Success(0);
        let res2 = res.clone();

        // reply to req on another thread
        let reply_thread = thread::spawn(move || {
            c2.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PTY, move |actual_req| {
                assert_eq!(actual_req, req2);
                res2
            })
            .unwrap();
        });

        // send req on main thread
        let actual_res = c1
            .send::<PtySlaveCall, PtySlaveResponse>(Channel::PTY, req)
            .unwrap();
        assert_eq!(actual_res, res);

        reply_thread.join().expect("failed to join reply thread");
    }

    #[test]
    fn test_send_receive_msg_loop() {
        let (t1, t2) = MemoryTransport::pair();

        let mut c1 = RemoteChannel::new(t1);
        let mut c2 = RemoteChannel::new(t2);
        let num_iters = 100;

        // reply to reqs on another thread
        let reply_thread = thread::spawn(move || {
            for i in 1..num_iters {
                c2.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PTY, move |actual_req| {
                    assert_eq!(
                        actual_req,
                        PtySlaveCall {
                            fd: Fd(i),
                            typ: PtySlaveCallType::GetAttr,
                        }
                    );

                    PtySlaveResponse::Success(i as _)
                })
                .unwrap();
            }
        });

        // send reqs on main thread
        for i in 1..num_iters {
            let actual_res = c1
                .send::<PtySlaveCall, PtySlaveResponse>(
                    Channel::PTY,
                    PtySlaveCall {
                        fd: Fd(i),
                        typ: PtySlaveCallType::GetAttr,
                    },
                )
                .unwrap();

            assert_eq!(actual_res, PtySlaveResponse::Success(i as _));
        }

        reply_thread.join().expect("failed to join reply thread");
    }

    #[test]
    fn test_send_receive_multiple_types() {
        let (t1, t2) = MemoryTransport::pair();

        let c1 = RemoteChannel::new(t1);
        let c2 = RemoteChannel::new(t2);

        let send_thread1 = {
            let mut c1 = c1.clone();
            thread::spawn(move || {
                let res = c1
                    .send::<PtySlaveCall, PtySlaveResponse>(
                        Channel::PTY,
                        PtySlaveCall {
                            fd: Fd(1),
                            typ: PtySlaveCallType::GetAttr,
                        },
                    )
                    .unwrap();

                assert_eq!(res, PtySlaveResponse::Success(1));
            })
        };

        let send_thread2 = {
            let mut c1 = c1.clone();
            thread::spawn(move || {
                let res = c1
                    .send::<PtyMasterCall, PtyMasterResponse>(
                        Channel::STDIN,
                        PtyMasterCall::Signal(PtyMasterSignal::SIGCONT),
                    )
                    .unwrap();

                assert_eq!(res, PtyMasterResponse::Success(2));
            })
        };

        let send_thread3 = {
            let mut c1 = c1.clone();
            thread::spawn(move || {
                let res = c1
                    .send::<PtyMasterCall, PtyMasterResponse>(
                        Channel::STDOUT,
                        PtyMasterCall::WriteStdin(WriteStdinCall {
                            data: vec![1, 2, 3],
                        }),
                    )
                    .unwrap();

                assert_eq!(res, PtyMasterResponse::Success(3));
            })
        };

        let reply_thread1 = {
            let mut c2 = c2.clone();
            thread::spawn(move || {
                c2.receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PTY, move |actual_req| {
                    assert_eq!(
                        actual_req,
                        PtySlaveCall {
                            fd: Fd(1),
                            typ: PtySlaveCallType::GetAttr,
                        }
                    );

                    PtySlaveResponse::Success(1)
                })
                .unwrap();
            })
        };

        let reply_thread2 = {
            let mut c2 = c2.clone();
            thread::spawn(move || {
                c2.receive::<PtyMasterCall, PtyMasterResponse, _>(
                    Channel::STDIN,
                    move |actual_req| {
                        assert_eq!(actual_req, PtyMasterCall::Signal(PtyMasterSignal::SIGCONT));

                        PtyMasterResponse::Success(2)
                    },
                )
                .unwrap();
            })
        };

        let reply_thread3 = {
            let mut c2 = c2.clone();
            thread::spawn(move || {
                c2.receive::<PtyMasterCall, PtyMasterResponse, _>(
                    Channel::STDOUT,
                    move |actual_req| {
                        assert_eq!(
                            actual_req,
                            PtyMasterCall::WriteStdin(WriteStdinCall {
                                data: vec![1, 2, 3]
                            })
                        );

                        PtyMasterResponse::Success(3)
                    },
                )
                .unwrap();
            })
        };

        for t in [send_thread1, send_thread2, send_thread3] {
            t.join().expect("failed to join send thread");
        }
        for t in [reply_thread1, reply_thread2, reply_thread3] {
            t.join().expect("failed to join reply thread");
        }
    }

    #[test]
    fn test_send_receive_multiple_types_loop() {
        let (t1, t2) = MemoryTransport::pair();

        let c1 = RemoteChannel::new(t1);
        let c2 = RemoteChannel::new(t2);
        let num_iters = 1000;

        let send_thread1 = {
            let mut c1 = c1.clone();
            thread::spawn(move || {
                for i in 1..num_iters {
                    let res = c1
                        .send::<PtySlaveCall, PtySlaveResponse>(
                            Channel::PTY,
                            PtySlaveCall {
                                fd: Fd(i),
                                typ: PtySlaveCallType::GetAttr,
                            },
                        )
                        .unwrap();

                    assert_eq!(res, PtySlaveResponse::Success(i as _));
                }
            })
        };

        let send_thread2 = {
            let mut c1 = c1.clone();
            thread::spawn(move || {
                for i in 1..num_iters {
                    let res = c1
                        .send::<PtySlaveCall, PtySlaveResponse>(
                            Channel::STDIN,
                            PtySlaveCall {
                                fd: Fd(i),
                                typ: PtySlaveCallType::WriteStdout(WriteStdoutCall {
                                    data: vec![1, 2, 3],
                                }),
                            },
                        )
                        .unwrap();

                    assert_eq!(res, PtySlaveResponse::Success(i as _));
                }
            })
        };

        let send_thread3 = {
            let mut c1 = c1.clone();
            thread::spawn(move || {
                for i in 1..num_iters {
                    let res = c1
                        .send::<PtyMasterCall, PtyMasterResponse>(
                            Channel::STDOUT,
                            PtyMasterCall::WriteStdin(WriteStdinCall {
                                data: [i as u8; 1024].to_vec(),
                            }),
                        )
                        .unwrap();

                    assert_eq!(res, PtyMasterResponse::Success(i as _));
                }
            })
        };

        let reply_thread1 = {
            let mut c2 = c2.clone();
            thread::spawn(move || {
                for i in 1..num_iters {
                    c2.receive::<PtySlaveCall, PtySlaveResponse, _>(
                        Channel::PTY,
                        move |actual_req| {
                            assert_eq!(
                                actual_req,
                                PtySlaveCall {
                                    fd: Fd(i),
                                    typ: PtySlaveCallType::GetAttr,
                                }
                            );

                            PtySlaveResponse::Success(i as _)
                        },
                    )
                    .unwrap();
                }
            })
        };

        let reply_thread2 = {
            let mut c2 = c2.clone();
            thread::spawn(move || {
                for i in 1..num_iters {
                    c2.receive::<PtySlaveCall, PtySlaveResponse, _>(
                        Channel::STDIN,
                        move |actual_req| {
                            assert_eq!(
                                actual_req,
                                PtySlaveCall {
                                    fd: Fd(i),
                                    typ: PtySlaveCallType::WriteStdout(WriteStdoutCall {
                                        data: vec![1, 2, 3],
                                    })
                                }
                            );

                            PtySlaveResponse::Success(i as _)
                        },
                    )
                    .unwrap();
                }
            })
        };

        let reply_thread3 = {
            let mut c2 = c2.clone();
            thread::spawn(move || {
                for i in 1..num_iters {
                    c2.receive::<PtyMasterCall, PtyMasterResponse, _>(
                        Channel::STDOUT,
                        move |actual_req| {
                            assert_eq!(
                                actual_req,
                                PtyMasterCall::WriteStdin(WriteStdinCall {
                                    data: [i as u8; 1024].to_vec(),
                                })
                            );

                            PtyMasterResponse::Success(i as _)
                        },
                    )
                    .unwrap();
                }
            })
        };

        for t in [send_thread1, send_thread2, send_thread3] {
            t.join().expect("failed to join send thread");
        }
        for t in [reply_thread1, reply_thread2, reply_thread3] {
            t.join().expect("failed to join reply thread");
        }
    }
}
