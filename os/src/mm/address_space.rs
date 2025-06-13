use super::{
    address::VPNRange,
    frame_alloc,
    page_table::{PageTable, PageTableView},
    FrameTracker, PageTableEntry, PhysAddr, PhysPageNum, VirtAddr, VirtPageNum,
};
use crate::{
    config::{MEMORY_END, MMIO, PAGE_SIZE, TRAMPOLINE},
    println,
};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use bitflags::bitflags;
use core::{arch::asm, cmp::min};
use riscv::register::satp;

unsafe extern "C" {
    unsafe fn stext();
    unsafe fn etext();
    unsafe fn srodata();
    unsafe fn erodata();
    unsafe fn sdata();
    unsafe fn edata();
    unsafe fn sbss_with_stack();
    unsafe fn ebss();
    unsafe fn ekernel();
    unsafe fn strampoline();
}

/// For MapType::Identical, the address space does not have ownership of the physical page frames it maps to.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    /// The values of R/W/X/U should be identical to those defined in struct PTEFlags.
    pub struct Permission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub struct MemorySegment {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    permission: Permission,
}

impl MemorySegment {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        permission: Permission,
    ) -> Self {
        Self {
            vpn_range: VPNRange::new(start_va.floor(), end_va.ceil()),
            data_frames: BTreeMap::new(),
            map_type: map_type,
            permission: permission,
        }
    }

    pub fn from_other(other: &Self) -> Self {
        Self {
            vpn_range: VPNRange::new(other.vpn_range.get_start(), other.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: other.map_type,
            permission: other.permission,
        }
    }

    /// Add the page with VirtPageNum vpn to page_table (and self.data_frames if self.map_type == Maptype::Framed).
    /// The page should belong to self.
    fn map_page(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        page_table.map(vpn, ppn, self.permission);
    }

    /// Delete the page with VirtPageNum vpn from page_table (and self.data_frames if self.map_type == Maptype::Framed).
    /// The page should belong to self.
    fn unmap_page(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Identical => {}
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
        }
        page_table.unmap(vpn);
    }

    /// Add self to page_table (and self.data_frames if self.map_type == Maptype::Framed).
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_page(page_table, vpn);
        }
    }

    /// Delete self from page_table (and self.data_frames if self.map_type == Maptype::Framed).
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_page(page_table, vpn);
        }
    }

    /// Must be called after self is added to page_table.
    pub fn copy_data(&mut self, page_table_view: PageTableView, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        for vpn in self.vpn_range {
            let src = &data[start..min(data.len(), start + PAGE_SIZE)];
            let dest = &mut page_table_view
                .translate(vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dest.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= data.len() {
                break;
            }
        }
    }
}

pub struct AddressSpace {
    page_table: PageTable,
    segments: Vec<MemorySegment>,
}

impl AddressSpace {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            segments: Vec::new(),
        }
    }

    pub fn from_existed_user(user_space: &Self) -> Self {
        let mut address_space = Self::new_bare();
        address_space.map_trampoline();
        // Copy data sections/trap context/user stack
        for segment in user_space.segments.iter() {
            address_space.add_segment(MemorySegment::from_other(segment), None);
            for vpn in segment.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dest_ppn = address_space.translate(vpn).unwrap().ppn();
                dest_ppn
                    .get_bytes_array()
                    .copy_from_slice(src_ppn.get_bytes_array());
            }
        }
        address_space
    }

    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            Permission::R | Permission::X,
        );
    }

    /// Return the kernel address space without the kernel stacks.
    pub fn new_kernel() -> Self {
        let mut address_space = AddressSpace::new_bare();
        address_space.map_trampoline();
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        println!("mapping .text section");
        address_space.add_segment(
            MemorySegment::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                Permission::R | Permission::X,
            ),
            None,
        );
        println!("mapping .rodata section");
        address_space.add_segment(
            MemorySegment::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                Permission::R,
            ),
            None,
        );
        println!("mapping .data section");
        address_space.add_segment(
            MemorySegment::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                Permission::R | Permission::W,
            ),
            None,
        );
        println!("mapping .bss section");
        address_space.add_segment(
            MemorySegment::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                Permission::R | Permission::W,
            ),
            None,
        );
        println!("mapping physical memory");
        address_space.add_segment(
            MemorySegment::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                Permission::R | Permission::W,
            ),
            None,
        );
        println!("mapping memory-mapped registers");
        for pair in MMIO {
            address_space.add_segment(
                MemorySegment::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identical,
                    Permission::R | Permission::W,
                ),
                None,
            );
        }
        address_space
    }

    /// Return (address_space, user_stack_base, the entry point of the program).
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut address_space = AddressSpace::new_bare();
        // User address space does not have the ownership of the physical frame where the trampoline code resides.
        // So the trampoline should only be added to the page table.
        address_space.map_trampoline();
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let magic = elf.header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf.header.pt2.ph_count();
        let mut segment_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut permission = Permission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    permission |= Permission::R;
                }
                if ph_flags.is_write() {
                    permission |= Permission::W;
                }
                if ph_flags.is_execute() {
                    permission |= Permission::X;
                }
                let segment = MemorySegment::new(start_va, end_va, MapType::Framed, permission);
                segment_end_vpn = segment.vpn_range.get_end();
                address_space.add_segment(
                    segment,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        let segment_end_va: VirtAddr = segment_end_vpn.into();
        // guard page between segment_end and user_stack
        let user_stack_base = segment_end_va.0 + PAGE_SIZE;
        (
            address_space,
            user_stack_base,
            elf.header.pt2.entry_point() as usize,
        )
    }

    fn add_segment(&mut self, mut segment: MemorySegment, data: Option<&[u8]>) {
        segment.map(&mut self.page_table);
        if let Some(data) = data {
            segment.copy_data(self.page_table.view(), data);
        }
        self.segments.push(segment);
    }

    pub fn add_segment_framed(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: Permission,
    ) {
        self.add_segment(
            MemorySegment::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    pub fn remove_segment_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((id, segment)) = self
            .segments
            .iter_mut()
            .enumerate()
            .find(|(_, segment)| segment.vpn_range.get_start() == start_vpn)
        {
            segment.unmap(&mut self.page_table);
            self.segments.remove(id);
        }
    }

    pub fn recycle_data_pages(&mut self) {
        self.segments.clear();
    }

    pub fn satp(&self) -> usize {
        self.page_table.satp()
    }

    pub fn activate(&self) {
        unsafe {
            satp::write(self.satp());
            asm!("sfence.vma");
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.view().translate(vpn)
    }
}

#[allow(unused)]
#[unsafe(no_mangle)]
pub fn remap_test() {
    let mut kernel_space = crate::mm::KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(!kernel_space
        .page_table
        .view()
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .view()
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .view()
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    println!("remap_test passed!");
}
