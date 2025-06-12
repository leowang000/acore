#![no_std]

use crate::linked_list::LinkedList;
use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::{max, min},
    ops::Deref,
    ptr::NonNull,
};
use spin::Mutex;

mod linked_list;

pub struct BuddyAllocator {
    /// free_list[i] contains the blocks of size (core::sizeof::<usize>() << i).
    free_list: [LinkedList; 32],
}

fn prev_power_of_two(num: usize) -> usize {
    let next_power = num.next_power_of_two();
    if num == next_power {
        next_power
    } else {
        next_power >> 1
    }
}

impl BuddyAllocator {
    pub const fn empty() -> Self {
        Self {
            free_list: [LinkedList::new(); 32],
        }
    }

    /// Add a range of memory [start, start + size) to the heap.
    pub unsafe fn init(&mut self, mut start: usize, size: usize) {
        // Avoid unaligned access.
        start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        let end = (start + size) & (!size_of::<usize>() + 1);
        assert!(start <= end);
        while start < end {
            let lowbit = start & (!start + 1);
            let size = min(lowbit, prev_power_of_two(end - start));
            self.free_list[size.trailing_zeros() as usize].push(start as *mut usize);
            start += size;
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        if let Some(i) = (class..self.free_list.len()).find(|i| !self.free_list[*i].is_empty()) {
            for j in (class + 1..=i).rev() {
                let block = self.free_list[j].pop().unwrap();
                self.free_list[j - 1].push(block);
                self.free_list[j - 1].push((block as usize + (1 << (j - 1))) as *mut usize);
            }
            if let Some(result) = NonNull::new(self.free_list[class].pop().unwrap() as *mut u8) {
                Ok(result)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let mut class = size.trailing_zeros() as usize;
        let mut ptr = ptr.as_ptr() as usize;
        self.free_list[class].push(ptr as *mut usize);
        while class < self.free_list.len() {
            let buddy = ptr ^ (1 << class);
            if let Some(block) = self.free_list[class]
                .iter_mut()
                .find(|block| block.value() as usize == buddy)
            {
                block.pop();
                self.free_list[class].pop();
                ptr = min(ptr, buddy);
                class += 1;
                self.free_list[class].push(ptr as *mut usize);
            } else {
                break;
            }
        }
    }
}

pub struct LockedBuddyAllocator {
    heap: Mutex<BuddyAllocator>,
}

impl LockedBuddyAllocator {
    pub const fn empty() -> Self {
        Self {
            heap: Mutex::new(BuddyAllocator::empty()),
        }
    }
}

impl Deref for LockedBuddyAllocator {
    type Target = Mutex<BuddyAllocator>;

    fn deref(&self) -> &Self::Target {
        &self.heap
    }
}

unsafe impl GlobalAlloc for LockedBuddyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock()
            .alloc(layout)
            .ok()
            .map_or(0 as *mut u8, |ptr| ptr.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(NonNull::new_unchecked(ptr), layout);
    }
}
