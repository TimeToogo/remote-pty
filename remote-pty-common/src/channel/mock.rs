use std::{
    fmt::Debug,
    thread::{self, JoinHandle},
};

use bincode::{Decode, Encode};

use super::{transport::mem::MemoryTransport, Channel, RemoteChannel};

pub struct MockChannel {
    pub chan: RemoteChannel,
    thread_handle: Option<JoinHandle<()>>,
}

impl MockChannel {
    fn pair() -> (RemoteChannel, RemoteChannel) {
        let (t1, t2) = MemoryTransport::pair();

        (RemoteChannel::new(t1), RemoteChannel::new(t2))
    }

    pub fn assert_sends<Req, Res>(
        chan: Channel,
        expected_reqs: Vec<Req>,
        expected_resp: Vec<Res>,
    ) -> Self
    where
        Req: Encode + Decode + Debug + Send + PartialEq + 'static,
        Res: Encode + Decode + Debug + Send + PartialEq + 'static,
    {
        let (c1, mut c2) = Self::pair();

        // assert and reply in new thread
        let thread_handle = thread::spawn(move || {
            for (req, res) in expected_reqs.into_iter().zip(expected_resp) {
                c2.receive::<Req, Res, _>(chan, move |actual_req| {
                    assert_eq!(actual_req, req);
                    res
                })
                .unwrap();
            }
        });

        Self {
            chan: c1,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn assert_receives<Req, Res>(
        chan: Channel,
        expected_reqs: Vec<Req>,
        expected_resp: Vec<Res>,
    ) -> Self
    where
        Req: Encode + Decode + Debug + Send + PartialEq + 'static,
        Res: Encode + Decode + Debug + Send + PartialEq + 'static,
    {
        let (c1, mut c2) = Self::pair();

        // assert and send in new thread
        let thread_handle = thread::spawn(move || {
            for (req, res) in expected_reqs.into_iter().zip(expected_resp) {
                let actual_res = c2.send::<Req, Res>(chan, req).unwrap();
                assert_eq!(actual_res, res);
            }
        });

        Self {
            chan: c1,
            thread_handle: Some(thread_handle),
        }
    }
}

impl Drop for MockChannel {
    fn drop(&mut self) {
        // ensure we validate the thread exits cleanly
        // when the test ends
        self.thread_handle.take().unwrap().join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::mock::MockChannel,
        proto::{
            slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
            Fd,
        },
    };

    use crate::channel::Channel;

    #[test]
    fn test_assert_sends() {
        let mut mock = MockChannel::assert_sends(
            Channel::PTY,
            vec![PtySlaveCall {
                fd: Fd(1),
                typ: PtySlaveCallType::GetAttr,
            }],
            vec![PtySlaveResponse::Success(1)],
        );

        let res = mock
            .chan
            .send::<_, PtySlaveResponse>(
                Channel::PTY,
                PtySlaveCall {
                    fd: Fd(1),
                    typ: PtySlaveCallType::GetAttr,
                },
            )
            .unwrap();

        assert_eq!(res, PtySlaveResponse::Success(1));
    }

    #[test]
    fn test_assert_receives() {
        let mut mock = MockChannel::assert_receives(
            Channel::PTY,
            vec![PtySlaveCall {
                fd: Fd(1),
                typ: PtySlaveCallType::GetAttr,
            }],
            vec![PtySlaveResponse::Success(1)],
        );

        mock.chan
            .receive::<PtySlaveCall, PtySlaveResponse, _>(Channel::PTY, |req| {
                assert_eq!(
                    req,
                    PtySlaveCall {
                        fd: Fd(1),
                        typ: PtySlaveCallType::GetAttr
                    }
                );
                PtySlaveResponse::Success(1)
            })
            .unwrap();
    }
}
