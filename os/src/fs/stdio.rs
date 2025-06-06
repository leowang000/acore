use crate::{
    fs::File, mm::UserBuffer, print, sbi::console_getchar, task::suspend_current_and_run_next,
};

pub struct Stdin;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        false
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        assert_eq!(buf.len(), 1);
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        unsafe { buf.buffers[0].as_mut_ptr().write_volatile(c as u8) };
        1
    }

    fn write(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
}

pub struct Stdout;

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }

    fn writable(&self) -> bool {
        true
    }

    fn read(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }

    fn write(&self, buf: UserBuffer) -> usize {
        for buffer in buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        buf.len()
    }
}
