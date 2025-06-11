use crate::{
    fs::{make_pipe, open_file, OpenFlags},
    mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer},
    task::{current_process, current_task_satp},
};

pub fn sys_dup(fd: usize) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(inner.fd_table[fd].as_ref().unwrap().clone());
    new_fd as isize
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let process = current_process();
    let satp = current_task_satp();
    let path = translated_str(satp, path);
    if let Some(inode) = open_file(&path, OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = process.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let process = current_process();
    let satp = current_task_satp();
    let mut inner = process.inner_exclusive_access();
    let (pipe_read, pip_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pip_write);
    *translated_refmut(satp, pipe) = read_fd;
    *translated_refmut(satp, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_write(fd: usize, buffer: *const u8, len: usize) -> isize {
    let satp = current_task_satp();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(satp, buffer, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buffer: *const u8, len: usize) -> isize {
    let satp = current_task_satp();
    let process = current_process();
    let inner = process.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.readable() {
            return -1;
        }
        let file = file.clone();
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(satp, buffer, len))) as isize
    } else {
        -1
    }
}
