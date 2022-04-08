use bincode::{Decode, Encode};

use crate::proto::{Termios, WinSize};

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtySlaveResponse {
    Success(i64),
    GetAttr(TcGetAttrResponse),
    GetWinSize(TcGetWinSizeResponse),
    Error(TcError),
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcgetattr.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcGetAttrResponse {
    pub ret: i64,
    pub termios: Termios,
}

// @see https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcGetWinSizeResponse {
    pub ret: i64,
    pub winsize: WinSize
}

// @see https://pubs.opengroup.org/onlinepubs/7908799/xsh/tcsetattr.html
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum TcError {
    EINVAL,
    EBADF,
    ENOTTY,
    EINTR,
    EIO,
}

#[cfg(test)]
mod tests {
    use crate::proto::{
        slave::{PtySlaveResponse, TcError},
    };

    #[test]
    fn encode_decode() {
        let config = bincode::config::standard();
        let get_attr_res = PtySlaveResponse::Error(TcError::EINVAL);

        assert_eq!(
            get_attr_res,
            bincode::decode_from_slice(
                bincode::encode_to_vec(get_attr_res.clone(), config)
                    .unwrap()
                    .as_slice(),
                config
            )
            .unwrap()
            .0
        );
    }
}
