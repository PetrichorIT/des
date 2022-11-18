use std::{
    alloc::{self, AllocError, Allocator, Layout},
    fmt::Debug,
    mem::{align_of, size_of},
    ptr::NonNull,
};

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct CQueueLLAllocatorInner {
    head: ListNode,
    pages: Vec<*mut u8>,
    page_size: usize,
    allocated_mem: usize,
}

impl CQueueLLAllocatorInner {
    /// Creates an empty LinkedListAllocator.
    pub fn new() -> Self {
        let mut this = Self {
            head: ListNode::new(0),
            pages: Vec::new(),
            page_size: page_size::get(),
            allocated_mem: 0,
        };

        unsafe {
            this.add_page();
        }

        this
    }

    pub(super) fn metrics(&self) -> (usize, usize) {
        (self.allocated_mem, self.page_size * self.page_size)
    }

    #[cfg(test)]
    pub(crate) fn info(&self) {}

    unsafe fn add_page(&mut self) {
        let block = alloc::alloc_zeroed(
            Layout::from_size_align(self.page_size, self.page_size).expect("page layout invalid"),
        );
        self.pages.push(block);
        self.add_free_region(block as usize, self.page_size)
    }

    pub fn handle(&self) -> CQueueLLAllocator {
        CQueueLLAllocator {
            inner: self as *const CQueueLLAllocatorInner as *mut CQueueLLAllocatorInner,
        }
    }

    #[cfg(test)]
    pub(crate) fn dbg_alloc_total(&self) -> usize {
        self.allocated_mem
    }

    #[cfg(test)]
    pub(crate) fn dbg_is_empty(&self) -> bool {
        self.allocated_mem == 0
    }

    #[cfg(test)]
    pub(crate) fn dbg_pages(&self) -> usize {
        self.pages.len()
    }

    /// Adds the given memory region to the front of the list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the freed region is capable of holding ListNode
        assert_eq!(align_up(addr, align_of::<ListNode>()), addr);
        assert!(size >= size_of::<ListNode>());

        // create a new list node and append it at the start of the list
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    /// Looks for a free region with the given size and alignment and removes
    /// it from the list.
    ///
    /// Returns a tuple of the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // reference to current list node, updated for each iteration
        let mut current = &mut self.head;
        // look for a large enough memory region in linked list
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // region suitable for allocation -> remove node from list
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // region not suitable -> continue with next region
                current = current.next.as_mut().unwrap();
            }
        }

        // TODO: maybe add to tail ?

        // no suitable region found
        // create new region
        unsafe {
            self.add_page();
            self.find_region(size, align)
        }
    }

    /// Try to use the given region for an allocation with given size and
    /// alignment.
    ///
    /// Returns the allocation start address on success.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < size_of::<ListNode>() {
            // rest of region too small to hold a ListNode (required because the
            // allocation splits the region in a used and a free part)
            return Err(());
        }

        // region suitable for allocation
        Ok(alloc_start)
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(size_of::<ListNode>());
        (size, layout.align())
    }
}

impl Debug for CQueueLLAllocatorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CQueueLLAllocatorInner")
            .field("allocated_memory", &self.allocated_mem)
            .field("pages", &self.pages.len())
            .finish()
    }
}

impl Drop for CQueueLLAllocatorInner {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.page_size, self.page_size)
            .expect("failed to generate page layout");
        for page in self.pages.iter() {
            unsafe { alloc::dealloc(*page, layout) }
        }
    }
}

// impl Allocator for LinkedListAllocator {}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CQueueLLAllocator {
    inner: *mut CQueueLLAllocatorInner,
}

unsafe impl Allocator for CQueueLLAllocator {
    fn allocate(&self, layout: std::alloc::Layout) -> Result<NonNull<[u8]>, AllocError> {
        let (size, align) = CQueueLLAllocatorInner::size_align(layout);
        let allocator = unsafe { &mut *self.inner };

        if size > allocator.page_size {
            return Err(AllocError);
        }

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            unsafe {
                let alloc_end = alloc_start.checked_add(size).expect("overflow");
                let excess_size = region.end_addr() - alloc_end;
                if excess_size > 0 {
                    if excess_size < size {
                        // println!("alloc: dropping {} bytes of memory", excess_size);
                        // alloc_end = alloc_end.checked_add(size).expect("overflow");
                    } else {
                        allocator.add_free_region(alloc_end, excess_size);
                    }
                }
                // println!(
                //     "alloc: Layout {{ size: {}, align: {} }} as Layout {{ size: {}, ptr: {} }}",
                //     layout.size(),
                //     layout.align(),
                //     size,
                //     alloc_start
                // );
                allocator.allocated_mem += size;
                Ok(NonNull::slice_from_raw_parts(
                    NonNull::new_unchecked(alloc_start as *mut u8),
                    size,
                ))
            }
        } else {
            Err(AllocError)
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let (size, _) = CQueueLLAllocatorInner::size_align(layout);
        let allocator = unsafe { &mut *self.inner };
        allocator.allocated_mem -= size;
        allocator.add_free_region(ptr.as_ptr() as usize, size)
    }
}

impl Debug for CQueueLLAllocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CQueueLLAllocator")
            .field("ptr", &self.inner)
            .finish()
    }
}
