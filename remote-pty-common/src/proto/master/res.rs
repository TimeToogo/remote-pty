use bincode::{Decode, Encode};

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtyMasterResponse {
    Success(i64),
    WriteSuccess,
    ReadSuccess(ReadResponse),
    Error(IoError),
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct ReadResponse {
    pub data: Vec<u8>
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum IoError {
    EINVAL,
    EBADF,
    ENOTTY,
    EINTR,
    EIO,
}
