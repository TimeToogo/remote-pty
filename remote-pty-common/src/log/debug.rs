use std::env;

pub fn debug<T>(msg: T) where T : Into<String> {
    if env::var("RPTY_DEBUG").is_ok() {
        println!("RPTY_DEBUG: {}", msg.into());
    }
}