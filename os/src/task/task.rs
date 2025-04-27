use super::TaskContext;

#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Clone, Copy)]
pub struct TaskControlBlock {
    pub status: TaskStatus,
    pub context: TaskContext,
}

impl TaskControlBlock {
    pub fn zero_init() -> Self {
        Self {
            status: TaskStatus::UnInit,
            context: TaskContext::zero_init(),
        }
    }
}
