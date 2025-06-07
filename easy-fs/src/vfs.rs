use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode, DiskInodeType, DIRENTRY_SZ},
};
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

pub struct Inode {
    disk_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        disk_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            disk_id: disk_id as usize,
            block_offset: block_offset,
            fs: fs,
            block_device: block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.disk_id, &self.block_device)
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.disk_id, &self.block_device)
            .lock()
            .modify(self.block_offset, f)
    }

    /// Find the inode_id of a file/directory inside the current directory (disk_inode) by name
    fn find_inode_id(
        name: &str,
        disk_inode: &DiskInode,
        block_device: &Arc<dyn BlockDevice>,
    ) -> Option<u32> {
        assert!(disk_inode.is_dir());
        let file_count = disk_inode.size as usize / DIRENTRY_SZ;
        let mut dir_entry = DirEntry::zero_init();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(i * DIRENTRY_SZ, dir_entry.as_bytes_mut(), block_device),
                DIRENTRY_SZ
            );
            if dir_entry.name() == name {
                return Some(dir_entry.inode_id() as u32);
            }
        }
        None
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            Self::find_inode_id(name, disk_inode, &self.block_device).map(|inode_id| {
                let (disk_id, block_offset) = fs.get_inode_block_disk_id(inode_id);
                Arc::new(Self::new(
                    disk_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            assert!(disk_inode.is_dir());
            let file_count = disk_inode.size as usize / DIRENTRY_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dir_entry = DirEntry::zero_init();
                assert_eq!(
                    disk_inode.read_at(
                        i * DIRENTRY_SZ,
                        dir_entry.as_bytes_mut(),
                        &self.block_device
                    ),
                    DIRENTRY_SZ
                );
                v.push(String::from(dir_entry.name()));
            }
            v
        })
    }

    fn increase_size(
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, block_device);
    }

    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self
            .modify_disk_inode(|disk_inode| {
                assert!(disk_inode.is_dir());
                Self::find_inode_id(name, &disk_inode, &self.block_device)
            })
            .is_some()
        {
            return None;
        }
        let new_inode_id = fs.alloc_inode();
        let (new_inode_disk_id, new_inode_block_offset) = fs.get_inode_block_disk_id(new_inode_id);
        get_block_cache(new_inode_disk_id as usize, &self.block_device)
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|disk_inode| {
            let file_count = disk_inode.size as usize / DIRENTRY_SZ;
            let new_size = (file_count + 1) * DIRENTRY_SZ;
            Self::increase_size(new_size as u32, disk_inode, &mut fs, &self.block_device);
            let dir_entry = DirEntry::new(name, new_inode_id);
            disk_inode.write_at(
                file_count * DIRENTRY_SZ,
                dir_entry.as_bytes(),
                &self.block_device,
            );
        });
        Some(Arc::new(Self::new(
            new_inode_disk_id,
            new_inode_block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let dealloc_data_blocks = disk_inode.clear_size(&self.block_device);
            assert!(dealloc_data_blocks.len() == DiskInode::total_blocks(size) as usize);
            for data_block in dealloc_data_blocks.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            Self::increase_size(
                (offset + buf.len()) as u32,
                disk_inode,
                &mut fs,
                &self.block_device,
            );
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }
}
