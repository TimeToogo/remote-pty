use remote_pty_common::{proto::slave::{PtySlaveResponse, SetProcGroupCall}, log::debug};

use crate::context::Context;

// since the process groups are remote we mock pgrp behaviour
pub fn handle_tcsetpgrp(ctx: &Context, req: SetProcGroupCall) -> PtySlaveResponse {
    let mut state = ctx.state.lock().expect("failed to lock terminal state");
    let _ = (*state).pgrp.insert(req.pid as _);

    debug(format!("set pgrp to {}", req.pid));
    PtySlaveResponse::Success(0)
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::{PtySlaveResponse, SetProcGroupCall};

    use crate::{context::Context, handler::handle_tcsetpgrp};

    #[test]
    fn test_tcsetpgrp_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        let req = SetProcGroupCall { pid: 123 };
        let ret = handle_tcsetpgrp(&ctx, req);

        let ret = match ret {
            PtySlaveResponse::Success(ret) => ret,
            res @ _ => {
                dbg!(res);
                unreachable!()
            }
        };

        assert_eq!(ret, 0);
        let state = ctx.state.lock().unwrap();
        assert_eq!((*state).pgrp, Some(123));
    }
}
