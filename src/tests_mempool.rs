// Apache License, Version 2.0
// (c) Campbell Barton, 2016

use std::ptr;
use mempool_elem::{
    MemPool,
    MemElemUtils,
};

struct TestElem {
    value: usize,
    link: *mut TestElem,
}

impl MemElemUtils for TestElem {
    fn free_ptr_get(&self) -> *mut TestElem {
        return self.link;
    }
    fn free_ptr_set(&mut self, ptr: *mut TestElem) {
        self.link = ptr;
    }
}

impl Default for TestElem {
    fn default() -> TestElem {
        TestElem {
            value: 0,
            link: ptr::null_mut(),
        }
    }
}

#[test]
fn test_mempool() {
    let total = 128;
    let chunk_size = 2;
    let mut p: MemPool<TestElem> = MemPool::new(chunk_size);

    for _ in 0..2 {
        let mut a = unsafe { &mut *p.alloc_elem() };
        a.value = 0;
        for i in 1..total {
            let a_next = p.alloc_elem();
            let a_prev = a;
            a = unsafe { &mut *a_next };
            a.value = i;
            a.link = a_prev;
        }

        for i in (0..total).rev() {
            assert!(a.value == i);
            let a_next = unsafe { &mut *a.link };
            p.free_elem(a);
            a = a_next;
        }
    }
}
