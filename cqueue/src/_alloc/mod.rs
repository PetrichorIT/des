use std::alloc::{AllocError, Allocator, Layout};
use std::marker::Destruct;
use std::ptr::NonNull;

#[cfg(test)]
mod tests;

const PAGE_SIZE: usize = 4096;

#[derive(Debug, Clone)]
pub(crate) struct CQueueAllocatorInner {
    head: *mut FreeNode,
    alloc_total: usize,
    blocks: Vec<Box<[u8; PAGE_SIZE]>>,
}

impl CQueueAllocatorInner {
    pub(crate) fn new() -> CQueueAllocatorInner {
        let mut block = Box::<[u8; PAGE_SIZE]>::new([0; PAGE_SIZE]);
        let head: *mut FreeNode = block.as_mut_ptr() as *mut FreeNode;
        unsafe { *head = FreeNode::new(PAGE_SIZE) };

        Self {
            head: head,
            alloc_total: 0,
            blocks: vec![block],
        }
    }

    #[cfg(test)]
    fn dbg_is_empty(&self) -> bool {
        self.alloc_total == 0
    }

    #[cfg(test)]
    fn dbg_alloc_total(&self) -> usize {
        self.alloc_total
    }

    unsafe fn get_mut_self(&self) -> &mut Self {
        let const_self: *const Self = &*self;
        let mut_self: *mut Self = const_self as *mut Self;
        let mut_self: &mut Self = unsafe { &mut *mut_self };
        mut_self
    }

    fn cleanup(&self) {
        if self.alloc_total != 0 {
            return;
        }
        let n = self.blocks.len();
        let mut prev = std::ptr::null_mut();
        for i in (0..n).rev() {
            let ptr = self.blocks[i].as_ptr() as *mut u8;
            let ptr = ptr as *mut FreeNode;
            unsafe {
                *ptr = FreeNode {
                    size: PAGE_SIZE,
                    next: prev,
                }
            }
            prev = ptr;
        }

        unsafe {
            self.get_mut_self().head = prev;
        }
    }

    pub fn handle(&self) -> CQueueAllocator {
        CQueueAllocator { ptr: self }
    }

    pub fn info(&self) {
        println!("CQueueAlloc::Info");
        let mut cur = self.head;
        while !cur.is_null() {
            unsafe {
                println!("Free block with {} bytes", (*cur).size);
                cur = (*cur).next
            }
        }
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(std::mem::align_of::<FreeNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(std::mem::size_of::<FreeNode>());
        (size, layout.align())
    }
}

unsafe impl Allocator for CQueueAllocatorInner {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let (size, align) = Self::size_align(layout);
        println!("alloc: {:?} -> {} / {}", layout, size, align);
        if size > 4096 {
            return Err(AllocError);
        }

        let const_self: *const Self = &*self;
        let mut_self: *mut Self = const_self as *mut Self;
        let mut_self: &mut Self = unsafe { &mut *mut_self };

        mut_self.alloc_total += size;

        let mut cur = self.head;

        loop {
            let ptr: *mut u8 = cur as *mut u8;
            let ptr_usize: usize = ptr as usize;

            let block_size = unsafe { (*cur).size };
            if block_size < size {
                if unsafe { (*cur).next.is_null() } {
                    // add new block
                    let mut block = Box::<[u8; PAGE_SIZE]>::new([0; PAGE_SIZE]);
                    let bhead: *mut FreeNode = block.as_mut_ptr() as *mut FreeNode;

                    unsafe {
                        *bhead = FreeNode {
                            size: PAGE_SIZE,
                            next: std::ptr::null_mut(),
                        }
                    }
                    mut_self.blocks.push(block);
                    unsafe {
                        (*cur).next = bhead;
                    }
                }
                cur = unsafe { (*cur).next };
            } else {
                let alloc_end = ptr_usize.checked_add(size).expect("overflow");
                let excess_size = block_size - size;
                if excess_size > 0 {
                    mut_self.head = alloc_end as *mut FreeNode;
                    unsafe {
                        *mut_self.head = FreeNode {
                            size: excess_size,
                            next: (*cur).next,
                        }
                    }

                    return Ok(NonNull::slice_from_raw_parts(
                        unsafe { NonNull::new_unchecked(ptr) },
                        size,
                    ));
                } else {
                    mut_self.head = unsafe { (*cur).next };
                    return Ok(NonNull::slice_from_raw_parts(
                        unsafe { NonNull::new_unchecked(ptr) },
                        size,
                    ));
                }
            }
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let (size, _) = Self::size_align(layout);
        let const_self: *const Self = &*self;
        let mut_self: *mut Self = const_self as *mut Self;
        let mut_self: &mut Self = unsafe { &mut *mut_self };

        mut_self.alloc_total -= size;

        let ptr: *mut FreeNode = ptr.as_ptr() as *mut FreeNode;
        unsafe {
            *ptr = FreeNode {
                size,
                next: self.head,
            }
        };

        mut_self.head = ptr;
    }
}

impl Drop for CQueueAllocatorInner {
    fn drop(&mut self) {
        println!("Dropping allocator");
        if self.alloc_total > 0 {
            eprintln!(
                "cqueue ** cqueue::alloc - dropping allocator without having dropped all values - rem {}",self.alloc_total
            )
        }
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

struct FreeNode {
    size: usize,
    next: *mut FreeNode,
}

impl FreeNode {
    const fn new(size: usize) -> Self {
        FreeNode {
            size,
            next: std::ptr::null_mut(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CQueueAllocator {
    ptr: *const CQueueAllocatorInner,
}

unsafe impl Allocator for CQueueAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe { (*self.ptr).allocate(layout) }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { (*self.ptr).deallocate(ptr, layout) }
    }
}

impl Destruct for CQueueAllocator {}
