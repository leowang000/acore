use crate::trap::trap_return;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskContext {
    sp: usize,
    ra: usize,
    s: [usize; 12],
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            sp: 0,
            ra: 0,
            s: [0; 12],
        }
    }

    pub fn goto_trap_return(kernel_stack_top: usize) -> Self {
        Self {
            sp: kernel_stack_top,
            ra: trap_return as usize,
            s: [0; 12],
        }
    }
}
