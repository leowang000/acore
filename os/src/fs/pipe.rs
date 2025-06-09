use crate::{fs::File, mm::UserBuffer, sync::UPSafeCell, task::suspend_current_and_run_next};
use alloc::sync::{Arc, Weak};

const RING_BUFFER_SIZE: usize = 32;

#[derive(Clone, Copy, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }

    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    /// The ring buffer must not be empty before read_byte is called.
    pub fn read_byte(&mut self) -> u8 {
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        self.status = if self.head == self.tail {
            RingBufferStatus::Empty
        } else {
            RingBufferStatus::Normal
        };
        c
    }

    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else {
            if self.tail > self.head {
                self.tail - self.head
            } else {
                self.tail + RING_BUFFER_SIZE - self.head
            }
        }
    }

    /// The ring buffer must not be full before write_byte is called.
    pub fn write_byte(&mut self, byte: u8) {
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        self.status = if self.tail == self.head {
            RingBufferStatus::Full
        } else {
            RingBufferStatus::Normal
        };
    }

    pub fn available_write(&self) -> usize {
        RING_BUFFER_SIZE - self.available_read()
    }

    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
    pub fn read_end_of_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer: buffer,
        }
    }

    pub fn write_end_of_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer: buffer,
        }
    }
}

impl File for Pipe {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, buf: UserBuffer) -> usize {
        assert!(self.readable());
        let buf_len = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_read: usize = 0;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_read = ring_buffer.available_read();
            if available_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return already_read;
                }
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..available_read {
                let byte_ref = buf_iter.next().unwrap();
                unsafe { *byte_ref = ring_buffer.read_byte() }
                already_read += 1;
                if already_read == buf_len {
                    return already_read;
                }
            }
        }
    }

    fn write(&self, buf: UserBuffer) -> usize {
        assert!(self.writable());
        let buf_len = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_write: usize = 0;
        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_write = ring_buffer.available_write();
            if available_write == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            for _ in 0..available_write {
                let byte_ref = buf_iter.next().unwrap();
                ring_buffer.write_byte(unsafe { *byte_ref });
                already_write += 1;
                if already_write == buf_len {
                    return already_write;
                }
            }
        }
    }
}

/// Return (read_end, write_end).
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(UPSafeCell::new(PipeRingBuffer::new()));
    let read_end = Arc::new(Pipe::read_end_of_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_of_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);
    (read_end, write_end)
}
