use riscv::register::sstatus::Sstatus;

#[repr(C)]
pub struct TrapContext {
    pub gprs: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    
}