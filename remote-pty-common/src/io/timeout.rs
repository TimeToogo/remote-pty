use std::{
    collections::HashMap,
    fmt::Debug,
    io,
    mem::{size_of, MaybeUninit},
    ptr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use lazy_static::lazy_static;

use crate::log::debug;

#[derive(Debug)]
pub enum TimeoutResult<R>
where
    R: Debug,
{
    Ok(R),
    Timeout(R),
    Error(io::Error),
}

// we keep a process-wide count of how threads have requested to install
// the handler for SIGALRM, this way we can restore it to the original value
// when it reaches zero
static WAITING_THREADS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    // we keep record of a monotonic "state" of the timeout
    // this number is incremented to signify a new timeout starting
    // or if it has completed
    static ref TIMEOUT_STATE: Mutex<HashMap<u64, AtomicU64>> = Mutex::new(HashMap::new());

    // capture the original signal handler for restoration when
    // no timeouts are active
    static ref ORIG_SIGACTION: Mutex<Option<libc::sigaction>> = Mutex::new(None);

    // a single worker thread responsible for processing all the timeouts
    static ref TIMEOUT_WORKER: TimeoutWorker = TimeoutWorker::spawn();
}

// waits a max duration for the blocking_op callback to complete
pub fn timeout<T, E>(
    duration: Duration,
    blocking_op: impl FnOnce() -> Result<T, E>,
) -> TimeoutResult<Result<T, E>>
where
    T: Debug,
    E: Debug,
{
    let tid = unsafe { libc::pthread_self() } as u64;

    // get the state for the current thread or set to 0 if
    // it is the first timeout
    let orig_state = thread_state(tid as _, |s| s.load(Ordering::SeqCst));

    // set the handler for SIGALRM
    // this will be restored when _sig_handler is dropped
    let _sig_handler = match SignalHandler::new() {
        Ok(h) => h,
        Err(_) => {
            debug("failed to create signal handler");
            return TimeoutResult::Error(io::Error::last_os_error());
        }
    };

    // create a timerfd and schedule it in the worker
    unsafe {
        let tfd =
            libc::timerfd_create(libc::CLOCK_REALTIME, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC);

        if tfd == -1 {
            debug("failed to create timerfd");
            return TimeoutResult::Error(io::Error::last_os_error());
        }

        let timerspec = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: duration.as_secs() as _,
                tv_nsec: duration.subsec_nanos() as _,
            },
        };

        let res = libc::timerfd_settime(
            tfd,
            0,
            &timerspec as *const _,
            ptr::null_mut::<libc::itimerspec>(),
        );

        if res == -1 {
            debug("failed to set timerfd time");
            return TimeoutResult::Error(io::Error::last_os_error());
        }

        let res = (*TIMEOUT_WORKER).schedule(TimeoutFd {
            tfd: tfd as _,
            tid,
            ts: orig_state,
        });

        if res == -1 {
            debug("failed to schedule timeout");
            return TimeoutResult::Error(io::Error::last_os_error());
        }
    }

    // here we perform the blocking operation
    let res = blocking_op();

    // if the state has changed since we received our SIGALRM
    // which means most likely the timeout expired
    let new_state = thread_state(tid, |s| s.load(Ordering::SeqCst));

    if new_state != orig_state {
        return TimeoutResult::Timeout(res);
    }

    // increment the state counter to signify we are complete
    // importantly this must occur before we restore the signal
    // handler so the timeout worker doesn't send SIGALRM after
    // we completed the op
    thread_state(tid, |s| s.fetch_add(1, Ordering::SeqCst));

    TimeoutResult::Ok(res)
}

struct SignalHandler {}

// overrides the SIGALRM handler to our handler function
impl SignalHandler {
    fn new() -> Result<Self, usize> {
        let mut orig = ORIG_SIGACTION.lock().unwrap();

        // only reset the signal handler if not present
        if orig.is_none() {
            unsafe {
                let mut orig_action = MaybeUninit::<libc::sigaction>::zeroed().assume_init();

                let mut new_action = MaybeUninit::<libc::sigaction>::zeroed().assume_init();
                new_action.sa_sigaction = signal_handler as *const fn(libc::c_int) as usize;

                let res = libc::sigaction(
                    libc::SIGALRM,
                    &mut new_action as *const _,
                    &mut orig_action as *mut _,
                );

                if res == -1 {
                    return Err(res as _);
                }

                let _ = orig.insert(orig_action);
            }
        }

        WAITING_THREADS.fetch_add(1, Ordering::SeqCst);
        Ok(Self {})
    }
}

// we restore the signal handler on drop
impl Drop for SignalHandler {
    fn drop(&mut self) {
        let mut orig = ORIG_SIGACTION.lock().unwrap();

        if WAITING_THREADS.fetch_sub(1, Ordering::SeqCst) > 1 {
            return;
        }

        let orig_action = orig.take().unwrap();
        unsafe {
            libc::sigaction(
                libc::SIGALRM,
                &orig_action as *const _,
                ptr::null_mut::<libc::sigaction>(),
            );
        }
    }
}

// our signal handler increments the thread's state value
extern "C" fn signal_handler(_signum: libc::c_int) {
    let tid = unsafe { libc::pthread_self() } as u64;

    thread_state(tid, |s| s.fetch_add(1, Ordering::SeqCst));
}

fn thread_state<R>(tid: u64, cb: impl FnOnce(&AtomicU64) -> R) -> R {
    let mut l = TIMEOUT_STATE.lock().unwrap();
    let s = l.entry(tid).or_insert_with(|| AtomicU64::new(0));

    cb(s)
}

// as long as size_of::<TimeoutFd>() < libc::PIPE_BUF
// we should be able to transmit this atomically over pipes
#[repr(C)]
struct TimeoutFd {
    tfd: u64, // timerfd
    tid: u64, // thread id
    ts: u64,  // timeout state at time of creation
}

struct TimeoutWorker {
    masterfd: libc::c_int,
    _handle: JoinHandle<()>,
}

// we try to support many concurrent timeouts in a scalable manner
// we do this in one thread using epoll
// @see https://man7.org/linux/man-pages/man7/epoll.7.html
impl TimeoutWorker {
    fn spawn() -> Self {
        assert!(size_of::<TimeoutFd>() <= libc::PIPE_BUF as _);

        let mut fds = [0 as libc::c_int; 2];
        let res = unsafe { libc::pipe2(&mut fds as *mut _, libc::O_CLOEXEC) };

        if res == -1 {
            debug("failed to create pipe");
            panic!("failed to create pipe");
        }

        let (readfd, writefd) = (fds[0], fds[1]);

        let handle = thread::spawn(move || unsafe {
            TimeoutWorker::start(readfd);
        });

        Self {
            masterfd: writefd,
            _handle: handle,
        }
    }

    unsafe fn schedule(&self, mut timeout: TimeoutFd) -> isize {
        // given size_of::<TimeoutFd>() < libc::PIPE_BUF this write must be atomic hence safe
        let res = libc::write(
            self.masterfd as _,
            &mut timeout as *mut TimeoutFd as *mut _,
            size_of::<TimeoutFd>(),
        );

        res
    }

    unsafe fn start(masterfd: libc::c_int) {
        let epfd = libc::epoll_create1(libc::O_CLOEXEC);

        if epfd == -1 {
            debug("failed to create epoll");
            return;
        }

        const MAX_EVENTS: usize = 10;
        let mut events = MaybeUninit::<[libc::epoll_event; MAX_EVENTS]>::zeroed().assume_init();

        // first we add the masterfd to epoll
        let mut ev = libc::epoll_event {
            events: libc::EPOLLIN as _,
            u64: masterfd as _,
        };
        let res = libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, masterfd, &mut ev as *mut _);

        if res == -1 {
            debug("failed to add master to epoll");
            return;
        }

        // hash map used for storing active timeouts
        let mut timeouts = HashMap::<u64, TimeoutFd, _>::new();

        // now we loop wait on the epoll
        loop {
            let nevs = libc::epoll_wait(epfd, &mut events as *mut _, MAX_EVENTS as _, -1 as _);

            if nevs == -1 {
                debug("failed to wait for epoll");
                return;
            }

            for i in 0usize..(nevs as usize) {
                let nev = events.get_unchecked(i);
                let rfd = nev.u64;

                // if we received a message on the master fd, it is a new timer fd
                if rfd == masterfd as _ {
                    let mut timeout = TimeoutFd {
                        tfd: 0,
                        tid: 0,
                        ts: 0,
                    };
                    // given size_of::<TimeoutFd>() < libc::PIPE_BUF this read must be atomic hence safe
                    let res = libc::read(
                        masterfd,
                        &mut timeout as *mut TimeoutFd as *mut _,
                        size_of::<TimeoutFd>(),
                    );

                    if res != size_of::<TimeoutFd>() as _ {
                        debug("failed to read new timeout from epoll");
                        return;
                    }

                    // add the new fd to the epoll
                    ev.events = libc::EPOLLIN as _;
                    ev.u64 = timeout.tfd;
                    let res = libc::epoll_ctl(
                        epfd,
                        libc::EPOLL_CTL_ADD,
                        timeout.tfd as _,
                        &mut ev as *mut _,
                    );

                    if res == -1 {
                        debug("failed to add new timerfd to epoll");
                        return;
                    }

                    timeouts.insert(timeout.tfd, timeout);
                } else {
                    // if it's not the master it must be a timeout which has expired
                    let timeout = timeouts.remove(&rfd).unwrap();

                    let cur_state = thread_state(timeout.tid, |s| s.load(Ordering::SeqCst));

                    // if the thread state has not changed, that means it is still blocked
                    // hence we send SIGALRM
                    // otherwise, the thread is no longer blocked on this op and we can safely ignore
                    if timeout.ts == cur_state {
                        // send the signal
                        let res = libc::pthread_kill(timeout.tid as libc::pthread_t, libc::SIGALRM);

                        // the thread could have terminated already so we continue even
                        // if the kill fails
                        if res != 0 {
                            debug("failed to send SIGALRM to thread");
                        }
                    }

                    // finally remove the timer from the epoll
                    let res = libc::epoll_ctl(
                        epfd,
                        libc::EPOLL_CTL_DEL,
                        rfd as _,
                        ptr::null_mut::<libc::epoll_event>(),
                    );

                    if res == -1 {
                        debug("failed to remove timerfd from epoll");
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Result},
        mem::MaybeUninit,
        ptr, thread,
        time::Duration,
    };

    use rand::Rng;

    use super::{timeout, TimeoutResult};

    #[test]
    fn test_timeout_which_returns() {
        let res = timeout(Duration::from_millis(100), || {
            thread::sleep(Duration::from_millis(1));
            Ok(10) as Result<_>
        });

        match res {
            TimeoutResult::Ok(Ok(r)) => assert_eq!(r, 10),
            _ => {
                dbg!(res);
                unreachable!();
            }
        }
    }

    #[test]
    fn test_timeout_which_times_out() {
        let res = timeout(Duration::from_millis(100), || {
            thread::sleep(Duration::from_millis(200));
            Ok(()) as Result<_>
        });

        match res {
            TimeoutResult::Timeout(_) => {}
            _ => {
                dbg!(res);
                unreachable!();
            }
        }
    }

    #[test]
    fn test_timeout_which_times_out_read() {
        let res = timeout(Duration::from_millis(100), || {
            perform_blocking_read(200);
            Ok(()) as Result<_>
        });

        match res {
            TimeoutResult::Timeout(_) => {
                assert_eq!(
                    io::Error::last_os_error().raw_os_error().unwrap(),
                    libc::EINTR
                );
            }
            _ => {
                dbg!(res);
                unreachable!();
            }
        }
    }

    #[test]
    fn stress_test() {
        const THREADS: usize = 1000;
        let mut rng = rand::thread_rng();
        let mut threads = Vec::new();

        for _ in 1..THREADS {
            let timeout_duration = rng.gen_range::<u64, _>(500..1000);
            let should_timeout = rng.gen_bool(0.5);
            let sleep_duration = ((timeout_duration as i64)
                + rng.gen_range::<i64, _>(if should_timeout { 300..500 } else { -500..-300 }))
                as u64;

            let handle = thread::spawn(move || {
                let res = timeout(Duration::from_millis(timeout_duration), || {
                    perform_blocking_read(sleep_duration as _);

                    Ok(()) as Result<_>
                });

                match (should_timeout, res) {
                    (true, TimeoutResult::Timeout(_)) => {}
                    (false, TimeoutResult::Ok(_)) => {}
                    (_, res) => {
                        dbg!(should_timeout);
                        dbg!(res);
                        unreachable!();
                    }
                }
            });

            threads.push(handle);
        }

        for i in threads {
            i.join().unwrap();
        }
    }

    fn perform_blocking_read(millis: usize) {
        lazy_static::lazy_static! {
            static ref BLOCKED_PIPE: [libc::c_int; 2] = unsafe {
                let mut pipe = [0 as libc::c_int; 2];
                libc::pipe2(&mut pipe as *mut _, libc::O_CLOEXEC);
                pipe
            };
        }

        unsafe {
            let fds = MaybeUninit::<libc::fd_set>::zeroed().as_mut_ptr();
            libc::FD_SET(BLOCKED_PIPE[1], fds);

            let mut timeval = libc::timeval {
                tv_sec: 0,
                tv_usec: (millis * 1_000) as i64,
            };

            libc::select(
                BLOCKED_PIPE[1] + 1,
                fds,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut timeval as *mut _,
            );
        }
    }
}
