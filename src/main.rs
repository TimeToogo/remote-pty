fn main() {
    ioctl();
}


use core::arch::asm;

#[no_mangle]
extern "C" fn ioctl() {
    // println!("== intercepted ==");
    let msg = "== intercepted ==\n".as_bytes();
    let res: i64;

    unsafe {
        asm!(
            "mov rax, {syscall}",
            "mov rdi, {fd}",
            "mov rsi, {buf}",
            "mov rdx, {len}",
            "syscall",
            syscall = in(reg) 1 as i64, // write
            fd = in(reg) 1 as u64, // stdout
            buf = in(reg) &msg[0] as *const u8,
            len = in(reg) msg.len() as usize,
            lateout("rax") res, // result
            out("rdi") _, // clobbered
            out("rsi") _, // clobbered
            out("rdx") _, // clobbered
            lateout("rcx") _, // clobbered
            lateout("r11") _, // clobbered
        );
    }
    
    println!("res = {}", res);
    println!("last OS error: {:?}", std::io::Error::last_os_error());
}