use alloc::vec::Vec;
use alloc::{string::String, vec};
use bitflags::bitflags;

use super::PhysAddr;
use super::{
    address::{PhysPageNum, StepByOne, VirtPageNum},
    address_space::Permission,
    frame_allocator::{frame_alloc, FrameTracker},
    VirtAddr,
};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits << 10 >> 20).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        let id = vpn.indexes();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[id[i]];
            if i == 2 {
                return Some(pte);
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        None
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        PageTableView::from_page_table(self).find_pte(vpn)
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, permission: Permission) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(
            ppn,
            PTEFlags::from_bits(permission.bits()).unwrap() | PTEFlags::V,
        );
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn view(&self) -> PageTableView {
        PageTableView {
            root_ppn: self.root_ppn,
        }
    }

    pub fn satp(&self) -> usize {
        0b1000usize << 60 | self.root_ppn.0
    }
}

pub struct PageTableView {
    root_ppn: PhysPageNum,
}

impl PageTableView {
    pub fn from_satp(satp: usize) -> Self {
        Self {
            root_ppn: satp.into(),
        }
    }

    pub fn from_page_table(page_table: &PageTable) -> Self {
        Self {
            root_ppn: page_table.root_ppn,
        }
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        let id = vpn.indexes();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[id[i]];
            if i == 2 {
                return Some(pte);
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        None
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| pte.clone())
    }

    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.floor())
            .map(|pte| (usize::from(PhysAddr::from(pte.ppn())) + va.page_offset()).into())
    }

    #[allow(unused)]
    pub fn satp(&self) -> usize {
        0b1000usize << 60 | self.root_ppn.0
    }
}

pub fn translated_byte_buffer(satp: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table_view = PageTableView::from_satp(satp);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table_view.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = core::cmp::min(end_va, end.into());
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

pub fn translated_str(satp: usize, ptr: *const u8) -> String {
    let page_table_view = PageTableView::from_satp(satp);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *page_table_view.translate_va(va.into()).unwrap().get_mut();
        if ch == b'\0' {
            return string;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
}

pub fn translated_ref<T>(satp: usize, ptr: *const T) -> &'static T {
    PageTableView::from_satp(satp)
        .translate_va((ptr as usize).into())
        .unwrap()
        .get_ref()
}

pub fn translated_refmut<T>(satp: usize, ptr: *mut T) -> &'static mut T {
    PageTableView::from_satp(satp)
        .translate_va((ptr as usize).into())
        .unwrap()
        .get_mut()
}

/// Abstract the result of translated_byte_buffer as &[u8].
pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers: buffers }
    }

    pub fn len(&self) -> usize {
        let mut length: usize = 0;
        for b in self.buffers.iter() {
            length += b.len();
        }
        length
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            buffers: self.buffers,
            buffer_id: 0,
            offset: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    buffer_id: usize,
    offset: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer_id >= self.buffers.len() {
            None
        } else {
            let cur = &mut self.buffers[self.buffer_id][self.offset] as *mut _;
            if self.offset + 1 == self.buffers[self.buffer_id].len() {
                self.buffer_id += 1;
                self.offset = 0;
            } else {
                self.offset += 1;
            }
            Some(cur)
        }
    }
}
