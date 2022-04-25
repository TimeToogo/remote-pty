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
mod tcdrain;
pub use tcdrain::*;
mod tcflow;
pub use tcflow::*;
mod tcflush;
pub use tcflush::*;
mod tcsendbreak;
pub use tcsendbreak::*;
mod ioctl;
pub use ioctl::*;

use remote_pty_common::{
    log::debug,
    proto::slave::{PtySlaveCall, PtySlaveCallType, PtySlaveResponse},
};

use crate::context::Context;

pub struct RemotePtyHandlers;

impl RemotePtyHandlers {
    pub fn handle(ctx: &Context, req: PtySlaveCall) -> PtySlaveResponse {
        let res = match req.typ {
            PtySlaveCallType::RegisterProcess(_) => todo!(),
            PtySlaveCallType::SetProcessGroup(_) => todo!(),
            PtySlaveCallType::GetAttr => handle_tcgetattr(ctx),
            PtySlaveCallType::SetAttr(req) => handle_tcsetattr(ctx, req),
            PtySlaveCallType::Drain => handle_tcdrain(ctx),
            PtySlaveCallType::Flow(req) => handle_tcflow(ctx, req),
            PtySlaveCallType::Flush(req) => handle_tcflush(ctx, req),
            PtySlaveCallType::SendBreak(req) => handle_tcsendbreak(ctx, req),
            PtySlaveCallType::IsATty => handle_isatty(ctx),
            PtySlaveCallType::GetWinSize => handle_tcgetwinsize(ctx),
            PtySlaveCallType::SetWinSize(req) => handle_tcsetwinsize(ctx, req),
            PtySlaveCallType::Ioctl(req) => handle_ioctl(ctx, req),
            PtySlaveCallType::GetProcGroup => handle_tcgetpgrp(ctx),
            PtySlaveCallType::SetProgGroup(req) => handle_tcsetpgrp(ctx, req),
            PtySlaveCallType::WriteStdout(_) => todo!(),
        };
     
        debug(format!("response: {:?}", res));

        res
    }
}
