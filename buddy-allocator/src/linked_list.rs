#[derive(Clone, Copy)]
pub struct LinkedList {
    head: *mut usize,
}

impl LinkedList {
    pub const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    pub fn push(&mut self, item: *mut usize) {
        unsafe { *item = self.head as usize };
        self.head = item;
    }

    pub fn pop(&mut self) -> Option<*mut usize> {
        if self.is_empty() {
            None
        } else {
            let item = self.head;
            self.head = unsafe { *item as *mut usize };
            Some(item)
        }
    }

    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            cur: self.head,
            prev: &mut self.head as *mut _ as *mut usize,
        }
    }
}

unsafe impl Send for LinkedList {}

pub struct ListNode {
    cur: *mut usize,
    prev: *mut usize,
}

impl ListNode {
    pub fn pop(self) -> *mut usize {
        unsafe {
            *(self.prev) = *(self.cur);
        }
        self.cur
    }

    pub fn value(&self) -> *mut usize {
        self.cur
    }
}

pub struct IterMut {
    cur: *mut usize,
    prev: *mut usize,
}

impl Iterator for IterMut {
    type Item = ListNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur.is_null() {
            None
        } else {
            let res = ListNode {
                cur: self.cur,
                prev: self.prev,
            };
            self.prev = self.cur;
            self.cur = unsafe { *self.cur as *mut usize };
            Some(res)
        }
    }
}
