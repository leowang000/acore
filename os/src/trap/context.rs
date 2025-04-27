use riscv::register::sstatus::{self, Sstatus};

#[repr(C)]
pub struct TrapContext {
    pub gprs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn app_initial_context(user_stack_sp: usize, app_base_address: usize) -> Self {
        let mut gprs: [usize; 32] = [0; 32];
        gprs[2] = user_stack_sp;
        let mut sstatus = sstatus::read();
        sstatus.set_spp(sstatus::SPP::User);
        TrapContext {
            gprs: gprs,
            sstatus: sstatus,
            sepc: app_base_address,
        }
    }
}
