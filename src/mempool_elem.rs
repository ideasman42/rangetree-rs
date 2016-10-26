// Apache License, Version 2.0
// (c) Campbell Barton, 2016

/// This module handles many small allocations of the same type
/// using memory chunks and a single linked list for a free-chain of elements.
///
/// Users of this API need to define get/set methods
/// so they can be members of the free-chain.

use std::ptr;

pub trait MemElemUtils {
    fn free_ptr_get(&self) -> *mut Self;
    fn free_ptr_set(&mut self, ptr: *mut Self);
}

pub trait MemElem:
    MemElemUtils +
    Default +
    {}

impl<TElem> MemElem for TElem where TElem:
    MemElemUtils +
    Default +
    {}

struct MemChunk<TElem: MemElem> {
    data: Vec<TElem>,
}

pub struct MemPool<TElem: MemElem> {
    /// Data storage.
    chunks: Vec<MemChunk<TElem>>,
    /// Number of elements per chunk.
    chunk_size: usize,
    /// Single linked list of freed elements to be reused.
    /// `free_ptr_get` is used to store the *chain* terminating at `null`.
    free: *mut TElem,
}

impl <TElem: MemElem> MemPool<TElem> {
    pub fn new(
        chunk_size: usize,
    ) -> MemPool<TElem> {
        MemPool {
            chunks: vec![
                MemChunk {
                    data: Vec::with_capacity(chunk_size),
                },
            ],
            chunk_size: chunk_size,
            free: ptr::null_mut(),
        }
    }

    pub fn clear(
        &mut self,
    ) {
        self.chunks.truncate(1);
        self.chunks[0].data.clear();
        debug_assert!(self.chunks[0].data.capacity() == self.chunk_size);
        self.free = ptr::null_mut();
    }

    #[allow(dead_code)]
    pub fn alloc_elem(
        &mut self,
    ) -> *mut TElem {
        if self.free.is_null() {
            if self.chunks.last().unwrap().data.len() == self.chunk_size {
                self.chunks.push(MemChunk {
                    data: Vec::with_capacity(self.chunk_size),
                });
            }
            let chunk = self.chunks.last_mut().unwrap();
            chunk.data.push(TElem::default());
            return chunk.data.last_mut().unwrap();
        } else {
            let elem = self.free;
            self.free = unsafe { (*elem).free_ptr_get() };
            return unsafe { &mut *elem };
        }
    }

    pub fn alloc_elem_from(
        &mut self,
        from: TElem,
    ) -> *mut TElem {
        if self.free.is_null() {
            if self.chunks.last().unwrap().data.len() == self.chunk_size {
                self.chunks.push(MemChunk {
                    data: Vec::with_capacity(self.chunk_size),
                });
            }
            let chunk = self.chunks.last_mut().unwrap();
            chunk.data.push(from);
            return chunk.data.last_mut().unwrap();
        } else {
            let elem = self.free;
            unsafe {
                self.free = (*elem).free_ptr_get();
                (*elem) = from;
            }
            return unsafe { &mut *elem };
        }
    }

    pub fn free_elem(
        &mut self,
        elem: *mut TElem,
    ) {
        unsafe {
            (*elem).free_ptr_set(self.free);
        }
        self.free = elem;
    }
}
