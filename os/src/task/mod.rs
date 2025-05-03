use crate::{
    loader::{get_app_data, get_num_app},
    println,
    sbi::shutdown,
    sync::UPSafeCell,
    trap::TrapContext,
};
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
use task::*;

mod context;
mod switch;
mod task;

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    current_app: usize,
    tasks: Vec<TaskControlBlock>,
}

impl TaskManager {
    fn run_first(&self) -> ! {
        let mut task_manager = self.inner.exclusive_access();
        let switch_from = &mut TaskContext::zero_init() as *mut TaskContext;
        let switch_to = &task_manager.tasks[0].context as *const TaskContext;
        task_manager.current_app = 0;
        task_manager.tasks[0].status = TaskStatus::Running;
        drop(task_manager);
        unsafe {
            __switch(switch_from, switch_to);
        }
        unreachable!();
    }

    fn suspend_current(&self) {
        let mut task_manager = self.inner.exclusive_access();
        let current_app = task_manager.current_app;
        task_manager.tasks[current_app].status = TaskStatus::Ready;
    }

    fn exit_current(&self) {
        let mut task_manager = self.inner.exclusive_access();
        let current_app = task_manager.current_app;
        task_manager.tasks[current_app].status = TaskStatus::Exited;
    }

    fn run_next(&self) {
        let mut task_manager = self.inner.exclusive_access();
        let next = (task_manager.current_app + 1..=task_manager.current_app + self.num_app)
            .map(|id| id % self.num_app)
            .find(|id| task_manager.tasks[*id].status == TaskStatus::Ready);
        if let Some(app_id) = next {
            let current_app = task_manager.current_app;
            let switch_from = &mut task_manager.tasks[current_app].context as *mut TaskContext;
            let switch_to = &task_manager.tasks[app_id].context as *const TaskContext;
            task_manager.current_app = app_id;
            task_manager.tasks[app_id].status = TaskStatus::Running;
            drop(task_manager);
            unsafe {
                __switch(switch_from, switch_to);
            }
        } else {
            println!("All tasks completed");
            shutdown(false);
        }
    }

    fn get_current_satp(&self) -> usize {
        let task_manager = self.inner.exclusive_access();
        task_manager.tasks[task_manager.current_app].satp()
    }

    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let task_manager = self.inner.exclusive_access();
        task_manager.tasks[task_manager.current_app].get_trap_cx()
    }

    fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut task_manager = self.inner.exclusive_access();
        let current_app = task_manager.current_app;
        task_manager.tasks[current_app].change_program_brk(size)
    } 
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app: num_app,
            inner: UPSafeCell::new(TaskManagerInner {
                current_app: 0,
                tasks: tasks,
            }),
        }
    };
}

pub fn run_first() {
    TASK_MANAGER.run_first();
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.suspend_current();
    TASK_MANAGER.run_next();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.exit_current();
    TASK_MANAGER.run_next();
}

pub fn current_user_satp() -> usize {
    TASK_MANAGER.get_current_satp()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}