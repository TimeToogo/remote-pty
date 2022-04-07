use std::env;

pub fn debug<T>(msg: T) where T : Into<String> {
    if env::var("REMOTE_PTY_DEBUG").is_ok() {
        println!("REMOTE_PTY_DEBUG: {}", msg.into());
    }
}