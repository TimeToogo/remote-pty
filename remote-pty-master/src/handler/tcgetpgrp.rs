use remote_pty_common::{proto::{
    slave::{PtySlaveResponse, ProcGroupResponse},
}, log::debug};

use crate::context::Context;

// since the process groups are remote we mock pgrp behaviour
pub fn handle_tcgetpgrp(ctx: &Context) -> PtySlaveResponse {
    let state = ctx.state.lock().expect("failed to lock terminal state");

    debug(format!("returned current pgrp {}", (*state).pgrp));
    PtySlaveResponse::GetProcGroup(ProcGroupResponse {
        pid: (*state).pgrp as _
    })
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse};

    use crate::{context::Context, handler::handle_tcgetpgrp};

    #[test]
    fn test_tcgetpgrp_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        {
            let mut state = ctx.state.lock().unwrap();
            state.pgrp = 123;
        }
        let ret = handle_tcgetpgrp(&ctx);

        let res = match ret {
            PtySlaveResponse::GetProcGroup(res) => res,
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        };

        assert_eq!(res.pid, 123);
    }
}
