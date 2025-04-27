use crate::{config::*, task::TaskContext, trap::TrapContext};
use core::arch::asm;

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

impl KernelStack {
    fn get_initial_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    // returns the kernel stack sp after pushing the trap context
    fn push_trap_context(&self, trap_context: TrapContext) -> usize {
        let kernel_stack_sp = self.get_initial_sp() - size_of::<TrapContext>();
        unsafe {
            *(kernel_stack_sp as *mut TrapContext) = trap_context;
        }
        kernel_stack_sp
    }
}

impl UserStack {
    fn get_initial_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

pub fn get_num_app() -> usize {
    unsafe extern "C" {
        unsafe fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDRESS + APP_SIZE_LIMIT * app_id
}

pub fn load_apps() {
    unsafe extern "C" {
        unsafe fn _num_app();
    }
    let num_app = get_num_app();
    let num_app_ptr = _num_app as usize as *const usize;
    let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
    let app_start_raw: &[usize] =
        unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    app_start[..=num_app].copy_from_slice(app_start_raw);
    for app_id in 0..num_app {
        let app_base_addr = get_base_i(app_id);
        unsafe {
            core::slice::from_raw_parts_mut(app_base_addr as *mut u8, APP_SIZE_LIMIT).fill(0);
        }
        let app_src = unsafe {
            core::slice::from_raw_parts(
                app_start[app_id] as *const u8,
                app_start[app_id + 1] - app_start[app_id],
            )
        };
        let app_dest =
            unsafe { core::slice::from_raw_parts_mut(app_base_addr as *mut u8, app_src.len()) };
        app_dest.copy_from_slice(app_src);
        unsafe {
            asm!("fence.i");
        }
    }
}

pub fn app_initial_context(app_id: usize) -> TaskContext {
    TaskContext::goto_restore(KERNEL_STACK[app_id].push_trap_context(
        TrapContext::app_initial_context(USER_STACK[app_id].get_initial_sp(), get_base_i(app_id)),
    ))
}
