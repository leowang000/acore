#![no_std]
#![feature(linkage)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::vec::Vec;
use bitflags::bitflags;
use buddy_allocator::LockedBuddyAllocator;
use syscall::*;

pub mod console;

mod lang_items;
mod syscall;

const USER_HEAP_SIZE: usize = 0x8000;
static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP_ALLOCATOR: LockedBuddyAllocator = LockedBuddyAllocator::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(&raw const HEAP_SPACE as usize, USER_HEAP_SIZE);
    }
    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == b'\0' })
            .unwrap();
        // The args in v do not end with '\0'.
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    exit(main(argc, v.as_slice()));
}

#[linkage = "weak"]
#[unsafe(no_mangle)]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}

pub const SIGDEF: i32 = 0;
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

bitflags! {
    pub struct SignalFlags: i32 {
        const SIGDEF = 1 << SIGDEF;
        const SIGHUP = 1 << SIGHUP;
        const SIGINT = 1 << SIGINT;
        const SIGQUIT = 1 << SIGQUIT;
        const SIGILL = 1 << SIGILL;
        const SIGTRAP = 1 << SIGTRAP;
        const SIGABRT = 1 << SIGABRT;
        const SIGBUS = 1 << SIGBUS;
        const SIGFPE = 1 << SIGFPE;
        const SIGKILL = 1 << SIGKILL;
        const SIGUSR1 = 1 << SIGUSR1;
        const SIGSEGV = 1 << SIGSEGV;
        const SIGUSR2 = 1 << SIGUSR2;
        const SIGPIPE = 1 << SIGPIPE;
        const SIGALRM = 1 << SIGALRM;
        const SIGTERM = 1 << SIGTERM;
        const SIGSTKFLT = 1 << SIGSTKFLT;
        const SIGCHLD = 1 << SIGCHLD;
        const SIGCONT = 1 << SIGCONT;
        const SIGSTOP = 1 << SIGSTOP;
        const SIGTSTP = 1 << SIGTSTP;
        const SIGTTIN = 1 << SIGTTIN;
        const SIGTTOU = 1 << SIGTTOU;
        const SIGURG = 1 << SIGURG;
        const SIGXCPU = 1 << SIGXCPU;
        const SIGXFSZ = 1 << SIGXFSZ;
        const SIGVTALRM = 1 << SIGVTALRM;
        const SIGPROF = 1 << SIGPROF;
        const SIGWINCH = 1 << SIGWINCH;
        const SIGIO = 1 << SIGIO;
        const SIGPWR = 1 << SIGPWR;
        const SIGSYS = 1 << SIGSYS;
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

impl Default for SignalAction {
    fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::empty(),
        }
    }
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}

pub fn sleep(sleep_ms: usize) {
    sys_sleep(sleep_ms);
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn kill(pid: usize, signum: i32) -> isize {
    sys_kill(pid, signum)
}

pub fn sigaction(
    signum: i32,
    action: Option<&SignalAction>,
    old_action: Option<&mut SignalAction>,
) -> isize {
    sys_sigaction(
        signum,
        action.map_or(core::ptr::null(), |action| action as *const _),
        old_action.map_or(core::ptr::null_mut(), |action| action as *mut _),
    )
}

pub fn sigprocmask(mask: u32) -> isize {
    sys_sigprocmask(mask)
}

pub fn sigreturn() -> isize {
    sys_sigreturn()
}

pub fn get_time() -> isize {
    sys_get_time()
}

pub fn getpid() -> isize {
    sys_getpid()
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid_nb(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _)
}

pub fn thread_create(entry: usize, arg: usize) -> isize {
    sys_thread_create(entry, arg)
}

pub fn gettid() -> isize {
    sys_gettid()
}

pub fn waittid(tid: usize) -> isize {
    loop {
        match sys_waittid(tid) {
            -2 => {
                yield_();
            }
            exit_code => return exit_code,
        }
    }
}

pub fn mutex_create() -> isize {
    sys_mutex_create(false)
}

pub fn mutex_blocking_create() -> isize {
    sys_mutex_create(true)
}

pub fn mutex_lock(mutex_id: usize) {
    sys_mutex_lock(mutex_id);
}

pub fn mutex_unlock(mutex_id: usize) {
    sys_mutex_unlock(mutex_id);
}

pub fn semaphore_create(res_count: usize) -> isize {
    sys_semaphore_create(res_count)
}

pub fn semaphore_up(sem_id: usize) {
    sys_semaphore_up(sem_id);
}

pub fn semaphore_down(sem_id: usize) {
    sys_semaphore_down(sem_id);
}

pub fn condvar_create() -> isize {
    sys_condvar_create()
}

pub fn condvar_signal(condvar_id: usize) {
    sys_condvar_signal(condvar_id);
}

pub fn condvar_wait(condvar_id: usize, mutex_id: usize) {
    sys_condvar_wait(condvar_id, mutex_id);
}

#[macro_export]
macro_rules! vstore {
    ($var: expr, $value: expr) => {
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!($var), $value);
        }
    };
}

#[macro_export]
macro_rules! vload {
    ($var: expr) => {
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!($var)) }
    };
}

#[macro_export]
macro_rules! memory_fence {
    () => {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst)
    };
}
