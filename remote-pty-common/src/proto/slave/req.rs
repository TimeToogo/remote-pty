use bincode::{Decode, Encode};

use crate::proto::{Fd, Termios, WinSize};

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/termios.h.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtySlaveCall {
    GetAttr(TcGetAttrCall),
    SetAttr(TcSetAttrCall),
    Drain(TcDrainCall),
    Flow(TcFlowCall),
    Flush(TcFlushCall),
    SendBreak(TcSendBreakCall),
    IsATty(IsATtyCall),
    GetWinSize(TcGetWinSizeCall),
    SetWinSize(TcSetWinSizeCall),
    Ioctl
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
    pub optional_actions: TcSetAttrActions,
    pub termios: Termios,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TcSetAttrActions {
    TCSANOW,
    TCSADRAIN,
    TCSAFLUSH,
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
    pub queue_selector: TcFlushQueueSelector,
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

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/isatty.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct IsATtyCall {
    pub fd: Fd,
}

// equivalent to ioctl(fd, TIOCGWINSZ, *winsize)
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcGetWinSizeCall {
    pub fd: Fd,
}

// equivalent to ioctl(fd, TIOCSWINSZ, *winsize)
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcSetWinSizeCall {
    pub fd: Fd,
    pub winsize: WinSize
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