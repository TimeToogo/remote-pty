use std::mem::MaybeUninit;

use remote_pty_common::log::debug;

pub(crate) fn get_inode_from_fd(fd: libc::c_int) -> Result<u64, String> {
    unsafe {
        let state = MaybeUninit::<libc::stat>::zeroed().as_mut_ptr();

        let res = libc::fstat(fd, state);

        if res != 0 {
            let msg = format!("failed to stat fd {}: {}", fd, errno::errno());
            debug(msg.clone());
            return Err(msg);
        }

        Ok((*state).st_ino as _)
    }
}

pub(crate) fn disable_input_buffering(_file: *mut libc::FILE) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    unsafe {
        let ret = libc::setvbuf(
            _file,
            std::ptr::null::<libc::c_char>() as *mut _,
            libc::_IONBF,
            0,
        );

        if ret != 0 {
            let msg = format!(
                "failed to disable input buffering on file: {}",
                errno::errno()
            );
            debug(msg.clone());
            return Err(msg);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{get_inode_from_fd};

    #[test]
    fn test_get_inode() {
        get_inode_from_fd(0).unwrap();
        get_inode_from_fd(-100).unwrap_err();
    }
}
