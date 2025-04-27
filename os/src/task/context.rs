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

    pub fn goto_restore(kernel_stack_sp: usize) -> Self {
        unsafe extern "C" {
            unsafe fn __restore();
        }
        Self {
            sp: kernel_stack_sp,
            ra: __restore as usize,
            s: [0; 12],
        }
    }
}
