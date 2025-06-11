use crate::task::signal::{SignalFlags, SIG_CNT};

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
            mask: SignalFlags::SIGQUIT | SignalFlags::SIGTRAP,
        }
    }
}

#[derive(Clone)]
pub struct SignalActionTable {
    pub table: [SignalAction; SIG_CNT],
}

impl Default for SignalActionTable {
    fn default() -> Self {
        Self {
            table: [SignalAction::default(); SIG_CNT],
        }
    }
}
