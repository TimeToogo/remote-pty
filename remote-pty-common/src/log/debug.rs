use std::{env, fs::OpenOptions, io::Write};

pub fn debug<T>(msg: T)
where
    T: Into<String>,
{
    let debug = env::var("RPTY_DEBUG");
    if debug.is_ok() {
        let mut f = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(debug.unwrap())
            .unwrap();
        writeln!(f, "RPTY_DEBUG: {}", msg.into()).unwrap();
    }
}
