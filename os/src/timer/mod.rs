use crate::{
    board::MTIMECMP,
    config::{CLOCK_FREQ, MTIME},
    sync::UPSafeCell,
    task::{wakeup_task, TaskControlBlock},
};
use alloc::{collections::binary_heap::BinaryHeap, sync::Arc};
use core::{arch::global_asm, cmp::Ordering};
use lazy_static::lazy_static;
use riscv::register::{mie, mscratch, mstatus, mtvec};

global_asm!(include_str!("timer_trap.S"));

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

#[link_section = ".bss.stack"]
static mut TIMER_SCRATCH: [usize; 5] = [0; 5];

pub fn init() {
    unsafe extern "C" {
        unsafe fn __timer_trap();
    }
    unsafe {
        TIMER_SCRATCH[3] = MTIMECMP;
        TIMER_SCRATCH[4] = CLOCK_FREQ / TICKS_PER_SEC;
        mtvec::write(__timer_trap as usize, mtvec::TrapMode::Direct);
        mscratch::write(&raw mut TIMER_SCRATCH as usize);
        mstatus::set_mie();
        mie::set_mtimer();
        (MTIMECMP as *mut usize).write_volatile(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
    }
}

/// Get the time in timer cycle count.
pub fn get_time() -> usize {
    unsafe { (MTIME as *const usize).read_volatile() }
}

/// Get the time in ms.
pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub struct TimerCondVar {
    pub expire_ms: usize,
    pub task: Arc<TaskControlBlock>,
}

impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}

impl Eq for TimerCondVar {}

impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.expire_ms.cmp(&self.expire_ms))
    }
}

impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    static ref TIMERS: UPSafeCell<BinaryHeap<TimerCondVar>> = UPSafeCell::new(BinaryHeap::new());
}

pub fn add_timer(expire_ms: usize, task: Arc<TaskControlBlock>) {
    TIMERS.exclusive_access().push(TimerCondVar {
        expire_ms: expire_ms,
        task: task,
    });
}

pub fn remove_timer(task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    let mut tmp = BinaryHeap::<TimerCondVar>::new();
    for condvar in timers.drain() {
        if !Arc::ptr_eq(&task, &condvar.task) {
            tmp.push(condvar);
        }
    }
    *timers = tmp;
}

pub fn check_timer() {
    let current_ms = get_time_ms();
    let mut timers = TIMERS.exclusive_access();
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= current_ms {
            wakeup_task(timer.task.clone());
            timers.pop();
        } else {
            break;
        }
    }
}
