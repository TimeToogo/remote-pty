use std::{env, fs::OpenOptions, io::Write};

pub fn debug<T>(msg: T)
where
    T: Into<String>,
{
    let debug = env::var("RPTY_DEBUG");
    if let Ok(f) = debug {
        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(f)
            .unwrap();
        let pid = unsafe { libc::getpid() };

        writeln!(f, "RPTY_DEBUG ({}): {}", pid, msg.into()).unwrap();
    }
}
