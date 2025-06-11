use crate::{sync::UPSafeCell, task::process::ProcessControlBlock};
use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PID2PCB: UPSafeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        UPSafeCell::new(BTreeMap::new());
}

pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    PID2PCB.exclusive_access().get(&pid).map(Arc::clone)
}

pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.exclusive_access().insert(pid, process);
}

pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.exclusive_access();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task", pid);
    }
}
