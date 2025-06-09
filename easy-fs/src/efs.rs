use crate::{
    bitmap::Bitmap,
    block_cache::{block_cache_sync_all, get_block_cache},
    block_dev::BlockDevice,
    layout::{DiskInode, DiskInodeType, SuperBlock},
    vfs::Inode,
    BLOCK_SZ,
};
use alloc::sync::Arc;
use spin::Mutex;

type DataBlock = [u8; BLOCK_SZ];

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}

impl EasyFileSystem {
    pub fn create(
        block_device: &Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.max_allocatable_blocks();
        let inode_area_blocks =
            (inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ;
        let inode_total_blocks = inode_bitmap_blocks as usize + inode_area_blocks;
        let data_total_blocks = total_blocks as usize - 1 - inode_total_blocks;
        let data_bitmap_blocks = (data_total_blocks + BLOCK_SZ * 8) / (BLOCK_SZ * 8 + 1);
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = Bitmap::new(1 + inode_total_blocks, data_bitmap_blocks);
        for i in 0..total_blocks {
            get_block_cache(i as usize, block_device).lock().modify(
                0,
                |data_block: &mut DataBlock| {
                    data_block.as_mut_slice().fill(0);
                },
            )
        }
        get_block_cache(0, block_device)
            .lock()
            .modify(0, |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks as u32,
                    data_bitmap_blocks as u32,
                    data_area_blocks as u32,
                );
            });
        let mut efs = Self {
            block_device: block_device.clone(),
            inode_bitmap: inode_bitmap,
            data_bitmap: data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: (1 + inode_total_blocks + data_bitmap_blocks) as u32,
        };
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_disk_id, root_inode_block_offset) = efs.get_inode_block_disk_id(0);
        get_block_cache(root_inode_disk_id as usize, block_device)
            .lock()
            .modify(root_inode_block_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }

    pub fn open(block_device: &Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, block_device)
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                Arc::new(Mutex::new(Self {
                    block_device: block_device.clone(),
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                }))
            })
    }

    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = efs.lock().block_device.clone();
        let (disk_id, block_offset) = efs.lock().get_inode_block_disk_id(0);
        Inode::new(disk_id, block_offset, efs.clone(), block_device)
    }

    /// Return the (disk_id, block_offset).
    pub fn get_inode_block_disk_id(&self, inode_block_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inode_per_block = BLOCK_SZ / inode_size;
        (
            self.inode_area_start_block + inode_block_id / inode_per_block as u32,
            inode_block_id as usize % inode_per_block * inode_size,
        )
    }

    pub fn get_data_block_disk_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }

    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    // dealloc_inode is not implemented because file deletion is not supported yet.

    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    pub fn dealloc_data(&mut self, disk_id: u32) {
        get_block_cache(disk_id as usize, &self.block_device)
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.as_mut_slice().fill(0);
            });
        assert!(disk_id >= self.data_area_start_block);
        self.data_bitmap.dealloc(
            &self.block_device,
            (disk_id - self.data_area_start_block) as usize,
        );
    }
}
