use bincode::{Decode, Encode};

use crate::proto::{Fd, Termios};

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/termios.h.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtySlaveCall {
    GetAttr(TcGetAttrCall),
    SetAttr(TcSetAttrCall),
    Drain(TcDrainCall),
    Flow(TcFlowCall),
    Flush(TcFlushCall),
    SendBreak(TcSendBreakCall),
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcgetattr.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcGetAttrCall {
    pub fd: Fd,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcsetattr.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcSetAttrCall {
    pub fd: Fd,
    pub termios: Termios,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcdrain.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcDrainCall {
    pub fd: Fd,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcflow.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcFlowCall {
    pub fd: Fd,
    pub action: TcFlowAction,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TcFlowAction {
    TCOOFF,
    TCOON,
    TCIOFF,
    TCION,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcflush.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcFlushCall {
    pub fd: Fd,
    pub action: TcFlushQueueSelector,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TcFlushQueueSelector {
    TCIFLUSH,
    TCOFLUSH,
    TCIOFLUSH,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcsendbreak.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcSendBreakCall {
    pub fd: Fd,
    pub duration: u32,
}

#[cfg(test)]
mod tests {
    use crate::proto::{slave::{PtySlaveCall, TcGetAttrCall}, Fd};

    #[test]
    fn encode_decode() {
        let config = bincode::config::standard();
        let get_attr_call = PtySlaveCall::GetAttr(TcGetAttrCall { fd: Fd(1) });

        assert_eq!(
            get_attr_call,
            bincode::decode_from_slice(
                bincode::encode_to_vec(get_attr_call.clone(), config)
                    .unwrap()
                    .as_slice(),
                config
            )
            .unwrap()
            .0
        );
    }
}