use crate::{
    config::TRAMPOLINE,
    println,
    syscall::syscall,
    task::{
        check_signals_of_current, current_add_signal, current_task_satp, current_task_trap_cx,
        current_task_trap_cx_user_va, exit_current_and_run_next, suspend_current_and_run_next,
        SignalFlags,
    },
    timer::{check_timer, set_next_trigger},
};
use core::arch::{asm, global_asm};
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sepc, sie, stval, stvec,
};

mod context;

pub use context::TrapContext;

global_asm!(include_str!("trap.S"));

#[unsafe(no_mangle)]
pub fn trap_from_kernel() -> ! {
    println!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, stvec::TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, stvec::TrapMode::Direct);
    }
}

pub fn init() {
    set_kernel_trap_entry();
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[unsafe(no_mangle)]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let cx = current_task_trap_cx();
            cx.sepc += 4;
            let result = syscall(cx.gprs[17], [cx.gprs[10], cx.gprs[11], cx.gprs[12]]) as usize;
            // trap_cx is changed during sys_exec, so we cannot use cx any more
            current_task_trap_cx().gprs[10] = result;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
            println!(
                "[debug] [kernel] trap {:?}, stval = {:#x}, sepc = {:#x}",
                scause.cause(),
                stval::read(),
                sepc::read()
            );
            current_add_signal(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            current_add_signal(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            check_timer();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    if let Some((errno, msg)) = check_signals_of_current() {
        println!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }
    trap_return();
}

#[unsafe(no_mangle)]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_user_va = current_task_trap_cx_user_va();
    let user_satp = current_task_satp();
    unsafe extern "C" {
        unsafe fn __alltraps();
        unsafe fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_user_va,
            in("a1") user_satp,
            options(noreturn)
        )
    }
}
