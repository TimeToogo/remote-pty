use std::{
    result::Result,
    sync::{Arc, Mutex},
};

use remote_pty_common::proto::slave::{PtySlaveCall, PtySlaveResponse};

use super::RemoteChannel;

// used for testing
pub struct MockChannel {
    data: Arc<Mutex<Data>>,
}

struct Data {
    requests: Vec<PtySlaveCall>,
    responses: Vec<PtySlaveResponse>,
}

impl MockChannel {
    pub fn new(
        mut expected_reqs: Vec<PtySlaveCall>,
        mut expected_resp: Vec<PtySlaveResponse>,
    ) -> Self {
        // we reverse the vecs so that we can use it as a stack
        // and pop off the starting elements first
        expected_reqs.reverse();
        expected_resp.reverse();

        return Self {
            data: Arc::new(Mutex::new(Data {
                requests: expected_reqs,
                responses: expected_resp,
            })),
        };
    }
}

impl RemoteChannel for MockChannel {
    fn send(&self, call: PtySlaveCall) -> Result<PtySlaveResponse, &'static str> {
        let mut data = self.data.lock().unwrap();

        assert_eq!(call, data.requests.pop().unwrap());

        return Ok(data.responses.pop().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::{
        slave::{PtySlaveCall, PtySlaveResponse, TcGetAttrCall},
        Fd,
    };

    use crate::channel::RemoteChannel;

    use super::MockChannel;

    #[test]
    fn test_mock_channel() {
        let chan = MockChannel::new(
            vec![PtySlaveCall::GetAttr(TcGetAttrCall { fd: Fd(1) })],
            vec![PtySlaveResponse::Success],
        );

        let res = chan.send(PtySlaveCall::GetAttr(TcGetAttrCall { fd: Fd(1) })).unwrap();

        assert_eq!(res, PtySlaveResponse::Success);
    }
}
