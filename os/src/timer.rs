use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

// get the time in timer cycle count
pub fn get_time() -> usize {
    time::read()
}

// get the time in ms
pub fn get_time_ms() -> usize {
    time::read() / CLOCK_FREQ * MSEC_PER_SEC
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
