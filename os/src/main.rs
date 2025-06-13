#![no_main]
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::arch::{asm, global_asm};
use riscv::register::{mepc, mstatus, pmpaddr0, pmpcfg0, satp};

pub mod fs;
pub mod lang_items;
pub mod mm;
pub mod sbi;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;

#[path = "boards/qemu.rs"]
mod board;
mod config;
mod console;
mod drivers;

global_asm!(include_str!("entry.asm"));

fn clear_bss() {
    unsafe extern "C" {
        unsafe fn sbss();
        unsafe fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[unsafe(no_mangle)]
unsafe fn rust_boot() -> ! {
    // Switch to S-mode after mret.
    mstatus::set_mpp(mstatus::MPP::Supervisor);
    // Jump to rust_main after mret.
    mepc::write(rust_main as usize);
    // Disable page table for M-mode.
    satp::write(0);
    // Delegate all possible exceptions to S-mode.
    // Allows S-mode to handle its own exceptions without M-mode intervention.
    asm!("csrw medeleg, {}", in(reg) 0xffff);
    // Delegate all possible interrupts to S-mode.
    // Enables S-mode to directly manage timer, software and external interrupts.
    asm!("csrw mideleg, {}", in(reg) 0xffff);
    // Set Physical Memory Protection address register 0, defining a vast protected region boundary
    // covering nearly the entire addressable physical memory space (0x3fffffffffffff = 2 ^ 54 - 1).
    pmpaddr0::write(0x3fffffffffffff);
    // Configure Physical Memory Protection settings via pmpcfg0 register.
    // Value 0xf (0b1111) enables Read, Write, Execute permissions with NAPOT addressing mode.
    pmpcfg0::write(0xf);
    // Init the timer.
    timer::init();
    asm!("mret", options(noreturn))
}

fn rust_init() {
    clear_bss();
    sbi::uart_init();
    mm::init();
    trap::init();
}

#[unsafe(no_mangle)]
fn rust_main() -> ! {
    rust_init();
    println!("[kernel] Hello, world!");
    fs::list_apps();
    task::run_tasks();
    unreachable!();
}
