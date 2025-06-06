use crate::{
    fs::{open_file, OpenFlags},
    mm::{translated_byte_buffer, translated_str, UserBuffer},
    task::{current_task, current_task_satp},
};

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let satp = current_task_satp();
    let path = translated_str(satp, path);
    if let Some(inode) = open_file(&path, OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_write(fd: usize, buffer: *const u8, len: usize) -> isize {
    let satp = current_task_satp();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(satp, buffer, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buffer: *const u8, len: usize) -> isize {
    let satp = current_task_satp();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(satp, buffer, len))) as isize
    } else {
        -1
    }
}
