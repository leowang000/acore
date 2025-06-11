use riscv::register::sstatus::{self, Sstatus};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub gprs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    // kernel_satp, kernel_stack_top, trap_handler are not part of the context.
    // They are initialized when app_initial_context is called, and their value will not change since then.
    pub kernel_satp: usize,
    /// the address of app kernel stack top in the kernel adress space
    pub kernel_stack_top: usize,
    pub trap_handler: usize,
}

impl TrapContext {
    pub fn app_initial_context(
        entry_point: usize, // va of the entry point of the user program
        user_sp: usize,
        kernel_satp: usize,
        kernel_stack_top: usize,
        trap_handler: usize,
    ) -> Self {
        let mut gprs: [usize; 32] = [0; 32];
        gprs[2] = user_sp; // set sp to user stack pointer
        let mut sstatus = sstatus::read();
        sstatus.set_spp(sstatus::SPP::User);
        TrapContext {
            gprs: gprs,
            sstatus: sstatus,
            sepc: entry_point,
            kernel_satp: kernel_satp,
            kernel_stack_top: kernel_stack_top,
            trap_handler: trap_handler,
        }
    }
}
