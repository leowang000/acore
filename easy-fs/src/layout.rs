use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SZ};
use alloc::{sync::Arc, vec::Vec};

const EFS_MAGIC: u32 = 0x3b800001;
/// the max number of direct inodes
const INODE_DIRECT_COUNT: usize = 28;
/// the max number of indirect1 inodes
const INODE_INDIRECT1_COUNT: usize = BLOCK_SZ / 4;
/// the max number of indirect2 inodes
const INODE_INDIRECT2_COUNT: usize = INODE_INDIRECT1_COUNT * INODE_INDIRECT1_COUNT;
/// the upper bound of direct inode index
const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
/// the upper bound of indirect1 inode index
const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_INDIRECT1_COUNT;
/// the upper bound of indirect2 inode index
#[allow(unused)]
const INDIRECT2_BOUND: usize = INDIRECT1_BOUND + INODE_INDIRECT2_COUNT;
/// the max length of inode name
const NAME_LENGTH_LIMIT: usize = 27;

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks: total_blocks,
            inode_bitmap_blocks: inode_bitmap_blocks,
            inode_area_blocks: inode_area_blocks,
            data_bitmap_blocks: data_bitmap_blocks,
            data_area_blocks: data_area_blocks,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

type IndirectBlock = [u32; INODE_INDIRECT1_COUNT];
type DataBlock = [u8; BLOCK_SZ];

#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}

impl DiskInode {
    pub fn initialize(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct.as_mut_slice().fill(0);
        self.indirect1 = 0;
        self.indirect2 = 0;
        self.type_ = type_;
    }

    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }

    #[allow(unused)]
    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }

    pub fn get_disk_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < DIRECT_BOUND {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, block_device)
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - DIRECT_BOUND]
                })
        } else {
            let indirect1 = get_block_cache(self.indirect2 as usize, block_device)
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[(inner_id - INDIRECT1_BOUND) / INODE_INDIRECT1_COUNT]
                });
            get_block_cache(indirect1 as usize, block_device)
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[(inner_id - INDIRECT1_BOUND) % INODE_INDIRECT1_COUNT]
                })
        }
    }

    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SZ as u32 - 1) / BLOCK_SZ as u32
    }

    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }

    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks;
        if data_blocks > DIRECT_BOUND {
            // indirect1
            total += 1;
        }
        if data_blocks > INDIRECT1_BOUND {
            // indirect2
            total += 1;
            // sub indirect1
            total +=
                (data_blocks - INDIRECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;
        }
        total as u32
    }

    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut current_blocks = self.data_blocks() as usize;
        self.size = new_size;
        let mut total_blocks = self.data_blocks() as usize;
        let mut new_blocks = new_blocks.into_iter();
        while current_blocks < core::cmp::min(INODE_DIRECT_COUNT, total_blocks) {
            self.direct[current_blocks] = new_blocks.next().unwrap();
            current_blocks += 1;
        }
        if total_blocks <= INODE_DIRECT_COUNT {
            return;
        }
        if current_blocks == INODE_DIRECT_COUNT {
            self.indirect1 = new_blocks.next().unwrap();
        }
        current_blocks -= INODE_DIRECT_COUNT;
        total_blocks -= INODE_DIRECT_COUNT;
        get_block_cache(self.indirect1 as usize, block_device)
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                while current_blocks < core::cmp::min(INODE_INDIRECT1_COUNT, total_blocks) {
                    indirect1[current_blocks] = new_blocks.next().unwrap();
                    current_blocks += 1;
                }
            });
        if total_blocks <= INODE_INDIRECT1_COUNT {
            return;
        }
        if current_blocks == INODE_INDIRECT1_COUNT {
            self.indirect2 = new_blocks.next().unwrap();
        }
        current_blocks -= INODE_INDIRECT1_COUNT;
        total_blocks -= INODE_INDIRECT1_COUNT;
        let mut a0 = current_blocks / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, block_device)
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                while (a0 < a1) || (a0 == a1 && b0 < b1) {
                    if b0 == 0 {
                        indirect2[a0] = new_blocks.next().unwrap();
                    }
                    get_block_cache(indirect2[a0] as usize, block_device)
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            indirect1[b0] = new_blocks.next().unwrap();
                        });
                    b0 += 1;
                    if b0 == INODE_INDIRECT1_COUNT {
                        b0 = 0;
                        a0 += 1;
                    }
                }
            });
    }

    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        let mut data_blocks = self.data_blocks() as usize;
        self.size = 0;
        let mut current_blocks = 0usize;
        while current_blocks < core::cmp::min(INODE_DIRECT_COUNT, data_blocks) {
            v.push(self.direct[current_blocks]);
            self.direct[current_blocks] = 0;
            current_blocks += 1;
        }
        if data_blocks <= INODE_DIRECT_COUNT {
            return v;
        }
        v.push(self.indirect1);
        current_blocks -= INODE_DIRECT_COUNT;
        data_blocks -= INODE_DIRECT_COUNT;
        get_block_cache(self.indirect1 as usize, block_device)
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                while current_blocks < core::cmp::min(INODE_INDIRECT1_COUNT, data_blocks) {
                    v.push(indirect1[current_blocks]);
                    current_blocks += 1;
                }
            });
        self.indirect1 = 0;
        if data_blocks <= INODE_INDIRECT1_COUNT {
            return v;
        }
        v.push(self.indirect2);
        data_blocks -= INODE_INDIRECT1_COUNT;
        let a1 = data_blocks / INODE_INDIRECT1_COUNT;
        let b1 = data_blocks % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, block_device)
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                for indirect1_id in indirect2.iter_mut().take(a1) {
                    v.push(*indirect1_id);
                    get_block_cache(*indirect1_id as usize, block_device)
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for disk_id in indirect1.iter() {
                                v.push(*disk_id);
                            }
                        })
                }
                if b1 > 0 {
                    v.push(indirect2[a1]);
                    get_block_cache(indirect2[a1] as usize, block_device)
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for disk_id in indirect1.iter().take(b1) {
                                v.push(*disk_id);
                            }
                        })
                }
            });
        self.indirect2 = 0;
        v
    }

    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut cur = offset;
        let end = core::cmp::min(offset + buf.len(), self.size as usize);
        if cur >= end {
            return 0;
        }
        let mut buf_ptr = 0usize;
        loop {
            let cur_end = core::cmp::min((cur / BLOCK_SZ + 1) * BLOCK_SZ, end);
            let block_read_bytes = cur_end - cur;
            let dst = &mut buf[buf_ptr..buf_ptr + block_read_bytes];
            get_block_cache(
                self.get_disk_id((cur / BLOCK_SZ) as u32, block_device) as usize,
                block_device,
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let src = &data_block[cur % BLOCK_SZ..cur % BLOCK_SZ + block_read_bytes];
                dst.copy_from_slice(src);
            });
            buf_ptr += block_read_bytes;
            if cur_end == end {
                break;
            }
            cur = cur_end;
        }
        buf_ptr
    }

    /// size must be adjusted properly beforehand
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut cur = offset;
        let end = core::cmp::min(offset + buf.len(), self.size as usize);
        assert!(cur <= end);
        let mut buf_ptr = 0usize;
        loop {
            let cur_end = core::cmp::min((cur / BLOCK_SZ + 1) * BLOCK_SZ, end);
            let block_write_bytes = cur_end - cur;
            let src = &buf[buf_ptr..buf_ptr + block_write_bytes];
            get_block_cache(
                self.get_disk_id((cur / BLOCK_SZ) as u32, block_device) as usize,
                block_device,
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                let dst = &mut data_block[cur % BLOCK_SZ..cur % BLOCK_SZ + block_write_bytes];
                dst.copy_from_slice(src);
            });
            buf_ptr += block_write_bytes;
            if cur_end == end {
                break;
            }
            cur = cur_end;
        }
        buf_ptr
    }
}

#[repr(C)]
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_id: u32,
}

/// the size of struct DirEnty
pub const DIRENTRY_SZ: usize = 32;

impl DirEntry {
    pub fn zero_init() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_id: 0,
        }
    }

    pub fn new(name: &str, inode_id: u32) -> Self {
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_id: inode_id,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENTRY_SZ) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENTRY_SZ) }
    }

    pub fn name(&self) -> &str {
        let len = (0usize..).find(|i| self.name[*i] == 0).unwrap();
        core::str::from_utf8(&self.name[..len]).unwrap()
    }

    pub fn inode_id(&self) -> u32 {
        self.inode_id
    }
}
