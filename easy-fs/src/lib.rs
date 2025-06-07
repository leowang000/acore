#![no_std]

extern crate alloc;

mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod layout;
mod vfs;

pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
pub use vfs::Inode;

/// the size of a block is 512 bytes
pub const BLOCK_SZ: usize = 512;
