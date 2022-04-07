// #![no_std]

use std::env;
use core::arch::asm;

#[no_mangle]
pub extern "C" fn ioctl() -> usize {
    let msg = "== intercepted ==\n".as_bytes();
    let res: u64;

    unsafe {
        asm!(
            "syscall",
            in("rax") 1, // write
            in("rdi") 1, // stdout
            in("rsi") &msg[0] as *const u8,
            in("rdx") msg.len(),
            lateout("rax") res, // result
            lateout("rcx") _, // clobbered
            lateout("r11") _, // clobbered
        );
    }

    return 0;
}


#[no_mangle]
pub extern "C" fn isatty() -> usize {
    let msg = "== isatty ==\n".as_bytes();
    let res: u64;

    let preload = env::var("test");

    unsafe {
        asm!(
            "syscall",
            in("rax") 1, // write
            in("rdi") 1, // stdout
            in("rsi") &msg[0] as *const u8,
            in("rdx") msg.len(),
            lateout("rax") res, // result
            lateout("rcx") _, // clobbered
            lateout("r11") _, // clobbered
        );
    }

    return 1;
}

use core::panic::PanicInfo;

// #[panic_handler]
// fn panic(_panic: &PanicInfo<'_>) -> ! {
//     loop {}
// }
