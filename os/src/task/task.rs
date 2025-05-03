use super::TaskContext;
use crate::{
    config::{TRAP_CONTEXT, kernel_stack_position},
    mm::{AddressSpace, KERNEL_SPACE, Permission, PhysPageNum, VirtAddr},
    trap::{TrapContext, trap_handler},
};

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub status: TaskStatus,
    pub context: TaskContext,
    pub address_space: AddressSpace,
    pub trap_cx_ppn: PhysPageNum,
    #[allow(unused)]
    pub base_size: usize,
    pub heap_bottom: usize,
    pub program_brk: usize,
}

impl TaskControlBlock {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn satp(&self) -> usize {
        self.address_space.satp()
    }

    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (address_space, user_sp, entry_point) = AddressSpace::from_elf(elf_data);
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().add_segment_framed(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            Permission::R | Permission::W,
        );
        let trap_cx_ppn = address_space
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let trap_cx = TrapContext::app_initial_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().satp(),
            kernel_stack_top,
            trap_handler as usize,
        );
        *trap_cx_ppn.get_mut() = trap_cx;
        Self {
            status: TaskStatus::Ready,
            context: TaskContext::goto_trap_return(kernel_stack_top),
            address_space: address_space,
            trap_cx_ppn: trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
        }
    }

    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_brk = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.address_space.shrink_to(self.heap_bottom.into(), (new_brk as usize).into())
        } else {
            self.address_space.append_to(self.heap_bottom.into(), (new_brk as usize).into())
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_brk)
        } else {
            None
        }
    }
}
