use crate::{mm::translated_byte_buffer, print, task::current_user_satp};

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buffer: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_satp(), buffer, len);
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
