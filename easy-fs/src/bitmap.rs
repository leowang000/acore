use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;

const BLOCK_BITS: usize = BLOCK_SZ * 8;

type BitmapBlock = [u64; 64];

pub struct Bitmap {
    start_disk_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_disk_id: usize, blocks: usize) -> Self {
        Self {
            start_disk_id: start_disk_id,
            blocks: blocks,
        }
    }

    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_pos in 0..self.blocks {
            let pos = get_block_cache(block_pos + self.start_disk_id, block_device)
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    if let Some((bits64_pos, inner_pos)) = bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                        .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                    {
                        bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                        Some(block_pos * BLOCK_BITS + bits64_pos * 64 + inner_pos)
                    } else {
                        None
                    }
                });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_disk_id, block_device)
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] &= !(1u64 << inner_pos);
            });
    }

    /// Get the max number of allocatable blocks.
    pub fn max_allocatable_blocks(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}

fn decomposition(bit: usize) -> (usize, usize, usize) {
    (bit / BLOCK_BITS, bit % BLOCK_BITS / 64, bit % 64)
}
