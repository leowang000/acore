mod context;

use crate::{batch::run_next_app, println, syscall::syscall};
use core::arch::global_asm;
use riscv::register::{
    scause::{self, Exception, Trap},
    stval, stvec,
};

pub use context::TrapContext;

global_asm!(include_str!("trap.S"));

pub fn init() {
    unsafe extern "C" {
        unsafe fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
    }
}

#[unsafe(no_mangle)]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.gprs[10] = syscall(cx.gprs[17], [cx.gprs[10], cx.gprs[11], cx.gprs[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            run_next_app();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it");
            run_next_app();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}
