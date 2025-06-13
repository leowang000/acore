use crate::{
    fs::{File, Stdin, Stdout},
    mm::{kernel_satp, translated_refmut, AddressSpace},
    sync::{Condvar, Mutex, Semaphore, UPSafeCell},
    task::{
        add_task, scheduler::insert_into_pid2process, RecycleAllocator, SignalActionTable,
        SignalFlags, TaskControlBlock,
    },
    trap::{trap_handler, TrapContext},
};
use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::cell::RefMut;

mod pid;

pub use pid::{pid_alloc, PidHandle};

pub struct ProcessControlBlockInner {
    pub is_zombie: bool,
    pub address_space: AddressSpace,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub signals: SignalFlags,
    pub signal_actions: SignalActionTable,
    pub signal_mask: SignalFlags,
    pub handling_signal: isize,
    pub killed: bool,
    pub frozen: bool,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub tid_allocator: RecycleAllocator,
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
}

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    inner: UPSafeCell<ProcessControlBlockInner>,
}

impl ProcessControlBlockInner {
    pub fn alloc_tid(&mut self) -> usize {
        self.tid_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.tid_allocator.dealloc(tid);
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.iter().len()
    }

    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

impl ProcessControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // Create address space.
        let (address_space, user_stack_base, entry_point) = AddressSpace::from_elf(elf_data);
        // Create new process.
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: UPSafeCell::new(ProcessControlBlockInner {
                is_zombie: false,
                address_space: address_space,
                parent: None,
                children: Vec::new(),
                exit_code: 0,
                fd_table: vec![
                    Some(Arc::new(Stdin)),
                    Some(Arc::new(Stdout)),
                    Some(Arc::new(Stdout)),
                ],
                signals: SignalFlags::empty(),
                signal_actions: SignalActionTable::default(),
                signal_mask: SignalFlags::empty(),
                handling_signal: -1,
                killed: false,
                frozen: false,
                tasks: Vec::new(),
                tid_allocator: RecycleAllocator::new(),
                mutex_list: Vec::new(),
                semaphore_list: Vec::new(),
                condvar_list: Vec::new(),
            }),
        });
        // Create main thread.
        let task = Arc::new(TaskControlBlock::new(
            process.clone(),
            user_stack_base,
            true,
        ));
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        let user_stack_top = task_inner.user_resource.as_ref().unwrap().user_stack_top();
        let kernel_stack_top = task.kernel_stack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_initial_context(
            entry_point,
            user_stack_top,
            kernel_satp(),
            kernel_stack_top,
            trap_handler as usize,
        );
        // Add main thread to the new process.
        process
            .inner_exclusive_access()
            .tasks
            .push(Some(task.clone()));
        // Add main thread to the task manager.
        add_task(task);
        // Add the new process to the process manager.
        insert_into_pid2process(process.get_pid(), process.clone());
        // Return the new process.
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // Modify PCB.
        let (address_space, user_stack_base, entry_point) = AddressSpace::from_elf(elf_data);
        let satp = address_space.satp();
        self.inner_exclusive_access().address_space = address_space;
        // Modify TCB.
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.user_resource.as_mut().unwrap().user_stack_base = user_stack_base;
        task_inner
            .user_resource
            .as_mut()
            .unwrap()
            .alloc_user_resource();
        task_inner.trap_cx_ppn = task_inner.user_resource.as_mut().unwrap().trap_cx_ppn();
        let mut user_sp = task_inner.user_resource.as_mut().unwrap().user_stack_top();
        // Push arguments on user stack.
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<&'static mut usize> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    satp,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        // Write nullptr at the end of argv.
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(satp, p as *mut u8) = *c;
                p += 1;
            }
            // Write '\0' at the end of each arg.
            *translated_refmut(satp, p as *mut u8) = 0;
        }
        // Make the user_sp aligned to 8B.
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // Modify trap_cx.
        let mut trap_cx = TrapContext::app_initial_context(
            entry_point,
            user_sp,
            kernel_satp(),
            task.kernel_stack.get_top(),
            trap_handler as usize,
        );
        trap_cx.gprs[10] = args.len();
        trap_cx.gprs[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent_inner = self.inner_exclusive_access();
        assert_eq!(parent_inner.thread_count(), 1);
        // Clone parent's address space completely (including user_stack and trap_cx).
        let address_space = AddressSpace::from_existed_user(&parent_inner.address_space);
        // Create child process.
        let pid = pid_alloc();
        let new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> =
            parent_inner.fd_table.iter().cloned().collect();
        let process = Arc::new(Self {
            pid: pid,
            inner: UPSafeCell::new(ProcessControlBlockInner {
                is_zombie: false,
                address_space: address_space,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                exit_code: 0,
                fd_table: new_fd_table,
                signals: SignalFlags::empty(),
                signal_actions: parent_inner.signal_actions.clone(),
                signal_mask: parent_inner.signal_mask,
                handling_signal: -1,
                killed: false,
                frozen: false,
                tasks: Vec::new(),
                tid_allocator: RecycleAllocator::new(),
                mutex_list: Vec::new(),
                semaphore_list: Vec::new(),
                condvar_list: Vec::new(),
            }),
        });
        // Create main thread of child process.
        let task = Arc::new(TaskControlBlock::new(
            process.clone(),
            parent_inner
                .get_task(0)
                .inner_exclusive_access()
                .user_resource
                .as_ref()
                .unwrap()
                .user_stack_base,
            // There is no need to allocate the user_stack ant the trap_cx, since these two segments have
            // been added to the child process's address space in AddressSpace::from_existed_user.
            false,
        ));
        task.inner_exclusive_access().get_trap_cx().kernel_stack_top = task.kernel_stack.get_top();
        // Add child's main thread to child process.
        process
            .inner_exclusive_access()
            .tasks
            .push(Some(task.clone()));
        // Add child's main thread to the task manager.
        add_task(task);
        // Add child process to the process manager.
        insert_into_pid2process(process.get_pid(), process.clone());
        // Add child to parent's children.
        parent_inner.children.push(process.clone());
        // Return the child process
        process
    }
}
