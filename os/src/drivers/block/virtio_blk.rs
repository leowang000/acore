use crate::{
    board::MMIO,
    mm::{
        frame_alloc, frame_dealloc, kernel_satp, FrameTracker, PageTableView, PhysAddr,
        PhysPageNum, StepByOne,
    },
    sync::UPSafeCell,
};
use alloc::vec::Vec;
use easy_fs::BlockDevice;
use lazy_static::lazy_static;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

lazy_static! {
    static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> = UPSafeCell::new(Vec::new());
}

struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
        let mut ppn_base = PhysPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            QUEUE_FRAMES.exclusive_access().push(frame);
        }
        PhysAddr::from(ppn_base).into()
    }

    fn dma_dealloc(paddr: virtio_drivers::PhysAddr, pages: usize) -> i32 {
        let mut ppn_base: PhysPageNum = PhysAddr::from(paddr).into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            QUEUE_FRAMES.exclusive_access().pop();
            ppn_base.step();
        }
        0
    }

    fn phys_to_virt(paddr: virtio_drivers::PhysAddr) -> virtio_drivers::VirtAddr {
        paddr
    }

    fn virt_to_phys(vaddr: virtio_drivers::VirtAddr) -> virtio_drivers::PhysAddr {
        PageTableView::from_satp(kernel_satp())
            .translate_va(vaddr.into())
            .unwrap()
            .0
    }
}

const VIRT_IO_0: usize = MMIO[1].0;

pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static, VirtioHal>>);

impl VirtIOBlock {
    pub fn new() -> Self {
        Self(UPSafeCell::new(
            VirtIOBlk::new(unsafe { &mut *(VIRT_IO_0 as *mut VirtIOHeader) }).unwrap(),
        ))
    }
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, disk_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(disk_id, buf)
            .expect("Error when reading VirtIOBlk");
    }

    fn write_block(&self, disk_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(disk_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}
