use remote_pty_common::{
    log::debug,
    proto::slave::{ProcGroupResponse, PtySlaveResponse},
};

use crate::context::Context;

// since the process groups are remote we mock pgrp behaviour
pub fn handle_tcgetpgrp(ctx: &Context) -> PtySlaveResponse {
    let state = ctx.state.lock().expect("failed to lock terminal state");

    // @see https://man7.org/linux/man-pages/man3/tcsetpgrp.3.html
    // hacky: when there is no foreground process group tcgetpgrp
    // will return a positive number that isn't a valid process group id
    // we do our best here
    let pid = (*state).pgrp.unwrap_or(99999) as _;

    debug(format!("returned current pgrp {}", pid));

    PtySlaveResponse::GetProcGroup(ProcGroupResponse { pid })
}

#[cfg(test)]
mod tests {
    use remote_pty_common::proto::slave::PtySlaveResponse;

    use crate::{context::Context, handler::handle_tcgetpgrp};

    #[test]
    fn test_tcgetpgrp_with_valid_pty() {
        let ctx = Context::openpty().unwrap();
        {
            let mut state = ctx.state.lock().unwrap();
            state.pgrp = Some(123);
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
