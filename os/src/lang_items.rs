use crate::{println, sbi::shutdown, task::current_kernel_stack_top};
use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("[kernel] Panicked: {}", info.message());
    }
    unsafe {
        backtrace();
    }
    shutdown(true)
}

unsafe fn backtrace() {
    let mut fp: usize;
    let stop = current_kernel_stack_top();
    asm!("mv {}, s0", out(reg) fp);
    println!("---START BACKTRACE---");
    for i in 0..10 {
        if fp == stop {
            break;
        }
        println!("#{}:ra={:#x}", i, *((fp - 8) as *const usize));
        fp = *((fp - 16) as *const usize);
    }
    println!("---END   BACKTRACE---");
}
