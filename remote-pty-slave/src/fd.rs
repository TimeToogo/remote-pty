use std::{fs, mem::MaybeUninit};

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

pub(crate) fn get_open_fds() -> Result<Vec<libc::c_int>, String> {
    let paths = fs::read_dir("/proc/self/fd/")
        .map_err(|err| format!("failed to open /proc/self/fd/ dir: {}", err))?;

    Ok(paths
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read entry: {}", err))?
        .into_iter()
        .filter_map(|i| {
            i.file_name()
                .to_str()
                .ok_or_else(|| "could not read file name".to_string())
                .and_then(|n| {
                    Ok(n.parse::<libc::c_int>()
                        .map_err(|e| format!("could not parse {} into int: {}", n, e))?)
                })
                .ok()
        })
        .collect::<Vec<libc::c_int>>())
}

pub(crate) fn get_open_fds_by_inode(inode: u64) -> Result<Vec<libc::c_int>, String> {
    let fds = get_open_fds()?;
    let mut filtered = vec![];

    for fd in fds {
        if get_inode_from_fd(fd).ok() == Some(inode) {
            filtered.push(fd);
        }
    }

    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::{get_inode_from_fd, get_open_fds};

    #[test]
    fn test_get_inode() {
        get_inode_from_fd(0).unwrap();
        get_inode_from_fd(-100).unwrap_err();
    }

    #[test]
    fn test_get_open_fds() {
        let fds = get_open_fds().unwrap();
        assert!(fds.len() > 0);
    }
}
