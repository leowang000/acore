#![no_main]
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::arch::global_asm;

pub mod syscall;
pub mod task;
pub mod trap;

#[path = "boards/qemu.rs"]
mod board;

mod config;
mod console;
mod lang_items;
mod loader;
mod mm;
mod sbi;
mod sync;
mod timer;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

fn clear_bss() {
    unsafe extern "C" {
        unsafe fn sbss();
        unsafe fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    println!("[kernel] Hello, world!");
    mm::init();
    println!("[kernel] back to world");
    mm::test();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::run_first();
    unreachable!();
}