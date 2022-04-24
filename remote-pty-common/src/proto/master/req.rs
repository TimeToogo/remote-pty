use bincode::{Decode, Encode};

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtyMasterCall {
    Signal(SignalCall),
    WriteStdin(WriteStdinCall),
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct SignalCall {
    pub signal: PtyMasterSignal,
    pub pgrp: u32
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum PtyMasterSignal {
    SIGWINCH,
    SIGINT,
    SIGTERM,
    SIGCONT,
    SIGTTOU,
    SIGTTIN,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct WriteStdinCall {
    pub data: Vec<u8>
}
