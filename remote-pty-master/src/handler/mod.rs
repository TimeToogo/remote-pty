pub mod common;

mod isatty;
pub use isatty::*;
mod tcgetattr;
pub use tcgetattr::*;
mod tcsetattr;
pub use tcsetattr::*;
mod tcgetwinsize;
pub use tcgetwinsize::*;
mod tcsetwinsize;
pub use tcsetwinsize::*;
mod tcgetpgrp;
pub use tcgetpgrp::*;
mod tcsetpgrp;
pub use tcsetpgrp::*;

use remote_pty_common::{
    log::debug,
    proto::slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
};

use crate::context::Context;

pub struct RemotePtyServer;

impl RemotePtyServer {
    pub fn handle(ctx: &Context, req: PtySlaveCall) -> PtySlaveResponse {
        debug(format!("handling request: {:?}", req));

        let res = match req.typ {
            PtySlaveCallType::GetAttr => handle_tcgetattr(ctx),
            PtySlaveCallType::SetAttr(req) => handle_tcsetattr(ctx, req),
            PtySlaveCallType::Drain => todo!(),
            PtySlaveCallType::Flow(_) => todo!(),
            PtySlaveCallType::Flush(_) => todo!(),
            PtySlaveCallType::SendBreak(_) => todo!(),
            PtySlaveCallType::IsATty => handle_isatty(ctx),
            PtySlaveCallType::GetWinSize => handle_tcgetwinsize(ctx),
            PtySlaveCallType::SetWinSize(req) => handle_tcsetwinsize(ctx, req),
            PtySlaveCallType::Ioctl(_) => todo!(),
            PtySlaveCallType::GetProcGroup => handle_tcgetpgrp(ctx),
            PtySlaveCallType::SetProgGroup(_) => todo!(),
        };

        debug(format!("response: {:?}", res));

        res
    }
}
