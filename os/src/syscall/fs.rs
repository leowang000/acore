use crate::{
    mm::translated_byte_buffer,
    print,
    sbi::console_getchar,
    task::{current_task_satp, suspend_current_and_run_next},
};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buffer: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_task_satp(), buffer, len);
            for str in buffers {
                print!("{}", core::str::from_utf8(str).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buffer: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read");
            let mut c: usize;
            loop {
                c = console_getchar();
                match c {
                    0 => {
                        suspend_current_and_run_next();
                        continue;
                    }
                    _ => {
                        break;
                    }
                }
            }
            let mut buffers = translated_byte_buffer(current_task_satp(), buffer, len);
            unsafe { buffers[0].as_mut_ptr().write_volatile(c as u8) };
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}
