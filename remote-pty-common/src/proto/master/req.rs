use bincode::{Decode, Encode};

use crate::proto::{Fd, Termios, WinSize};


// // TODO
// #[derive(Encode, Decode, PartialEq, Debug, Clone)]
// pub enum PtyMasterCall {
//     Signal(PtyMasterSignal),
//     Write(),
// }

// #[derive(Encode, Decode, PartialEq, Debug, Clone)]
// pub enum PtyMasterSignal {
//     SIGWINCH
// }
