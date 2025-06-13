use crate::{config::VIRT_UART0, sync::UPSafeCell};
use bitflags::bitflags;
use core::sync::atomic::{AtomicU8, Ordering};
use lazy_static::lazy_static;

macro_rules! wait_for {
    ($cond:expr) => {
        while !$cond {
            core::hint::spin_loop();
        }
    };
}

bitflags! {
    struct InterruptEnable: u8 {
        const RX_AVAILABLE = 1 << 0;
        const TX_EMPTY = 1 << 1;
    }

    struct FifoControl: u8 {
        const ENABLE = 1 << 0;
        const CLEAR_RX_FIFO = 1 << 1;
        const CLEAR_TX_FIFO = 1 << 2;
        const TRIGGER_14 = 0b11 << 6;
    }

    struct LineControl: u8 {
        const DATA_8 = 0b11;
        const DLAB_ENABLE = 1 << 7;
    }

    struct ModemControl: u8 {
        const DATA_TERMINAL_READY = 1 << 0;
        const AUXILIARY_OUTPUT_2 = 1 << 3;
    }

    struct LineStatus: u8 {
        const INPUT_AVAILABLE = 1 << 0;
        const OUTPUT_EMPTY = 1 << 5;
    }
}

/// read port when DLAB = 0
#[repr(C)]
struct ReadPort {
    /// receiver buffer
    rbr: AtomicU8,
    /// interrupt enable
    ier: AtomicU8,
    /// interrupt identification
    iir: AtomicU8,
    /// line control
    lcr: AtomicU8,
    /// modem control
    mcr: AtomicU8,
    /// line status
    lsr: AtomicU8,
    /// modem status
    msr: AtomicU8,
    /// scratch
    scr: AtomicU8,
}

/// write port when DLAB = 0
#[repr(C)]
struct WritePort {
    /// transmitter holding
    thr: AtomicU8,
    /// interrupt enable
    ier: AtomicU8,
    /// FIFO control
    fcr: AtomicU8,
    /// line control
    lcr: AtomicU8,
    /// modem control
    mcr: AtomicU8,
    _factory_test: AtomicU8,
    _not_used: AtomicU8,
    /// scratch
    scr: AtomicU8,
}

struct UartRaw {
    base: usize,
}

/// 38.4 Kbps
const UART_DIVISOR: usize = 3;

impl UartRaw {
    fn new(base: usize) -> Self {
        Self { base: base }
    }

    fn read_port(&self) -> &'static mut ReadPort {
        unsafe { &mut *(self.base as *mut ReadPort) }
    }

    fn write_port(&self) -> &'static mut WritePort {
        unsafe { &mut *(self.base as *mut WritePort) }
    }

    fn init(&self) {
        let write_port = self.write_port();
        // Disable interrupts.
        write_port
            .ier
            .store(InterruptEnable::empty().bits, Ordering::Release);
        // Enable DLAB.
        write_port
            .lcr
            .store(LineControl::DLAB_ENABLE.bits, Ordering::Release);
        // Set dll (to set maximum speed of 38.4K).
        write_port.thr.store(UART_DIVISOR as u8, Ordering::Release);
        // Set dlm (to set maximum speed of 38.4K).
        write_port
            .ier
            .store((UART_DIVISOR >> 8) as u8, Ordering::Release);
        // Disable DLAB and set data word length to 8 bits.
        write_port
            .lcr
            .store(LineControl::DATA_8.bits, Ordering::Release);
        // Enable FIFO, clear TX/RX queues and set interrupt watermark at 14 bytes.
        write_port.fcr.store(
            (FifoControl::ENABLE | FifoControl::TRIGGER_14).bits,
            Ordering::Release,
        );
        // Mark data terminal ready, signal request to send and enable auxilliary output.
        write_port.mcr.store(
            (ModemControl::DATA_TERMINAL_READY | ModemControl::AUXILIARY_OUTPUT_2).bits,
            Ordering::Release,
        );
        // Enable interrupts.
        write_port.ier.store(
            (InterruptEnable::RX_AVAILABLE | InterruptEnable::TX_EMPTY).bits,
            Ordering::Release,
        );
    }

    fn send(&self, byte: u8) {
        let read_port = self.read_port();
        wait_for!((read_port.lsr.load(Ordering::Acquire) & LineStatus::OUTPUT_EMPTY.bits) != 0);
        self.write_port().thr.store(byte, Ordering::Release);
    }

    fn recv(&self) -> u8 {
        let read_port = self.read_port();
        wait_for!((read_port.lsr.load(Ordering::Acquire) & LineStatus::INPUT_AVAILABLE.bits) != 0);
        read_port.rbr.load(Ordering::Acquire)
    }
}

lazy_static! {
    static ref UART: UPSafeCell<UartRaw> = UPSafeCell::new(UartRaw::new(VIRT_UART0));
}

pub fn uart_init() {
    UART.exclusive_access().init();
}

pub fn uart_send(byte: u8) {
    UART.exclusive_access().send(byte);
}

pub fn uart_recv() -> u8 {
    UART.exclusive_access().recv()
}
