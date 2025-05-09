use super::{
    TaskContext,
    kernel_stack::KernelStack,
    pid::{PidHandle, pid_alloc},
};
use crate::{
    config::TRAP_CONTEXT,
    mm::{AddressSpace, KERNEL_SPACE, PhysPageNum, VirtAddr},
    sync::UPSafeCell,
    trap::{TrapContext, trap_handler},
};
use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use core::cell::RefMut;

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

pub struct TaskControlBlockInner {
    pub status: TaskStatus,
    pub task_cx: TaskContext,
    pub address_space: AddressSpace,
    pub trap_cx_ppn: PhysPageNum,
    // All application data (user program, user stack, etc.) are present in regions of the address space that are below `base_size` bytes.
    // So base_size specifies how much user data are stored in the memory.
    #[allow(unused)]
    pub base_size: usize,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn satp(&self) -> usize {
        self.address_space.satp()
    }

    pub fn is_zombie(&self) -> bool {
        self.status == TaskStatus::Zombie
    }
}

pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn new(elf_data: &[u8]) -> Self {
        let (address_space, user_sp, entry_point) = AddressSpace::from_elf(elf_data);
        let trap_cx_ppn = address_space
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        *trap_cx_ppn.get_mut() = TrapContext::app_initial_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().satp(),
            kernel_stack_top,
            trap_handler as usize,
        );
        Self {
            pid: pid_handle,
            kernel_stack: kernel_stack,
            inner: UPSafeCell::new(TaskControlBlockInner {
                status: TaskStatus::Ready,
                task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                address_space: address_space,
                trap_cx_ppn: trap_cx_ppn,
                base_size: user_sp,
                parent: None,
                children: Vec::new(),
                exit_code: 0,
            }),
        }
    }

    // The physical frame where the trap context is stored will change during exec.
    pub fn exec(&self, elf_data: &[u8]) {
        let (address_space, user_sp, entry_point) = AddressSpace::from_elf(elf_data);
        let trap_cx_ppn = address_space
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        *trap_cx_ppn.get_mut() = TrapContext::app_initial_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().satp(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        let mut inner = self.inner.exclusive_access();
        inner.address_space = address_space;
        inner.trap_cx_ppn = trap_cx_ppn;
        inner.base_size = user_sp;
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent_inner = self.inner_exclusive_access();
        let address_space = AddressSpace::from_existed_user(&parent_inner.address_space);
        let trap_cx_ppn = address_space
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        trap_cx_ppn.get_mut::<TrapContext>().kernel_sp = kernel_stack_top;
        let task_control_block = Arc::new(Self {
            pid: pid_handle,
            kernel_stack: kernel_stack,
            inner: UPSafeCell::new(TaskControlBlockInner {
                status: TaskStatus::Ready,
                task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                address_space: address_space,
                trap_cx_ppn: trap_cx_ppn,
                base_size: parent_inner.base_size,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
            }),
        });
        parent_inner.children.push(task_control_block.clone());
        task_control_block
    }
}
