mod context;
mod switch;
mod task;

use crate::{
    config::*,
    loader::{app_initial_context, get_num_app},
    println,
    sbi::shutdown,
    sync::UPSafeCell,
};
use lazy_static::*;
use switch::__switch;
use task::*;

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    current_app: usize,
    tasks: [TaskControlBlock; MAX_APP_NUM],
}

impl TaskManager {
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
            .map(|id| id % MAX_APP_NUM)
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
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        TaskManager {
            num_app: num_app,
            inner: UPSafeCell::new({
                let mut tasks = [TaskControlBlock::zero_init(); MAX_APP_NUM];
                for app_id in 0..num_app {
                    tasks[app_id].context = app_initial_context(app_id);
                    tasks[app_id].status = TaskStatus::Ready;
                }
                TaskManagerInner {
                    current_app: 0,
                    tasks: tasks,
                }
            }),
        }
    };
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.suspend_current();
    TASK_MANAGER.run_next();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.exit_current();
    TASK_MANAGER.run_next();
}

pub fn run_first() {
    TASK_MANAGER.run_first();
}
