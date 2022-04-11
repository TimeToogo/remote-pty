pub mod common;

mod isatty;
pub use isatty::*;
use remote_pty_common::{
    log::debug,
    proto::slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
};

use crate::context::Context;

pub struct RemotePtyServer;

impl RemotePtyServer {
    pub fn handle(ctx: Context, req: PtySlaveCall) -> PtySlaveResponse {
        debug(format!("handling request: {:?}", req));

        let res = match req.typ {
            PtySlaveCallType::GetAttr => todo!(),
            PtySlaveCallType::SetAttr(_) => todo!(),
            PtySlaveCallType::Drain => todo!(),
            PtySlaveCallType::Flow(_) => todo!(),
            PtySlaveCallType::Flush(_) => todo!(),
            PtySlaveCallType::SendBreak(_) => todo!(),
            PtySlaveCallType::IsATty => handle_isatty(ctx),
            PtySlaveCallType::GetWinSize => todo!(),
            PtySlaveCallType::SetWinSize(_) => todo!(),
            PtySlaveCallType::Ioctl(_) => todo!(),
        };

        debug(format!("response: {:?}", res));

        res
    }
}
