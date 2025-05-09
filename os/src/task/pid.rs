use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub struct PidHandle(pub usize);

pub struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    pub fn dealloc(&mut self, pid: &PidHandle) {
        assert!(pid.0 < self.current);
        assert!(
            self.recycled
                .iter()
                .find(|target| **target == pid.0)
                .is_none(),
            "pid {} has been deallocated!",
            pid.0
        );
        self.recycled.push(pid.0);
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> = UPSafeCell::new(PidAllocator::new());
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}
