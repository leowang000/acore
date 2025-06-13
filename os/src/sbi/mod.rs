use crate::{config::VIRT_TEST, sbi::uart::{uart_recv, uart_send}};

mod uart;

pub use uart::uart_init;

pub fn console_putchar(c: u8) {
    uart_send(c);
}

pub fn console_getchar() -> u8 {
    uart_recv()
}

const FINISHER_FAIL: u32 = 0x3333;
const FINISHER_PASS: u32 = 0x5555;

pub fn shutdown(failure: bool) -> ! {
    unsafe {
        (VIRT_TEST as *mut u32).write_volatile(if failure {
            FINISHER_FAIL
        } else {
            FINISHER_PASS
        });
    }
    unreachable!();
}
