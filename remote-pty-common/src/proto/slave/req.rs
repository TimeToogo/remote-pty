use bincode::{Decode, Encode};

use crate::proto::{Termios, WinSize, Fd};

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/termios.h.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct PtySlaveCall {
    pub fd: Fd,
    pub typ: PtySlaveCallType,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtySlaveCallType {
    RegisterProcess(RegisterProcessCall),
    // @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcgetattr.html
    GetAttr,
    SetAttr(TcSetAttrCall),
    // @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcdrain.html
    Drain,
    Flow(TcFlowCall),
    Flush(TcFlushCall),
    SendBreak(TcSendBreakCall),
    // @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/isatty.html
    IsATty,
    // equivalent to ioctl(fd, TIOCGWINSZ, *winsize)
    GetWinSize,
    SetWinSize(TcSetWinSizeCall),
    Ioctl(IoctlCall),
    // equivalent to ioctl(fd, TIOCGPGRP, *pgrp)
    // @see https://man7.org/linux/man-pages/man3/tcgetpgrp.3.html
    GetProcGroup,
    SetProgGroup(SetProcGroupCall),
    WriteStdout(WriteStdoutCall)
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct RegisterProcessCall {
    pub pid: u32,
    pub pgrp: u32,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcsetattr.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcSetAttrCall {
    pub optional_actions: TcSetAttrActions,
    pub termios: Termios,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TcSetAttrActions {
    TCSANOW,
    TCSADRAIN,
    TCSAFLUSH,
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcflow.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcFlowCall {
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
    pub duration: u32,
}

// equivalent to ioctl(fd, TIOCSWINSZ, *winsize)
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcSetWinSizeCall {
    pub winsize: WinSize
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum IoctlCall {
    FIONREAD,
    TIOCOUTQ,
    TIOCGETD,
    TIOCSETD(u32),
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct SetProcGroupCall {
    pub pid: u32
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct WriteStdoutCall {
    pub data: Vec<u8>
}

impl PtySlaveCallType {
    // determines if the calling process must be in the foreground
    // to perform this call
    pub fn must_be_foreground(&self) -> bool {
        match self {
            Self::GetAttr => true,
            Self::SetAttr(_) => true,
            Self::Drain => true,
            Self::Flow(_) => true,
            Self::Flush(_) => true,
            Self::SendBreak(_) => true,
            Self::SetWinSize(_) => true,
            Self::Ioctl(_) => true,
            Self::SetProgGroup(_) => true,
            Self::WriteStdout(_) => true,
            _ => false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::proto::{slave::{PtySlaveCall, PtySlaveCallType}, Fd};

    #[test]
    fn encode_decode() {
        let config = bincode::config::standard();
        let get_attr_call = PtySlaveCall { fd: Fd(1), typ: PtySlaveCallType::GetAttr};

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