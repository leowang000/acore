use crate::task::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

unsafe extern "C" {
    pub unsafe fn __switch(from: *mut TaskContext, to: *const TaskContext);
}
