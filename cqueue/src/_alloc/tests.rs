use super::*;

#[test]
fn single_page_one_alloc_one_allocator() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueAllocatorInner::new();
    let bu32 = Box::new_in(42u32, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu32);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueAllocatorInner::new();
    let bu64 = Box::new_in(42u64, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu64);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueAllocatorInner::new();
    let bu128 = Box::new_in(42u128, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu128);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    // Now layout will grow

    let alloc = CQueueAllocatorInner::new();
    let barray = Box::new_in([42u8; 55], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 56); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueAllocatorInner::new();
    let barray = Box::new_in([42u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 128); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
    drop(alloc);
}

#[test]
fn single_page_one_alloc_shared_allocator() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueAllocatorInner::new();
    let bu32 = Box::new_in(42u32, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu32);
    assert!(alloc.dbg_is_empty());

    let bu64 = Box::new_in(42u64, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu64);
    assert!(alloc.dbg_is_empty());

    let bu128 = Box::new_in(42u128, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu128);
    assert!(alloc.dbg_is_empty());

    // Now layout will grow

    let barray = Box::new_in([42u8; 55], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 56); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());

    let barray = Box::new_in([42u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 128); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
}

#[test]
fn single_page_one_alloc_shared_allocator_with_cleanup() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueAllocatorInner::new();
    let bu32 = Box::new_in(42u32, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu32);
    assert!(alloc.dbg_is_empty());

    let bu64 = Box::new_in(42u64, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu64);
    assert!(alloc.dbg_is_empty());

    let bu128 = Box::new_in(42u128, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu128);
    assert!(alloc.dbg_is_empty());

    // Now layout will grow

    let barray = Box::new_in([42u8; 55], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 56); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());

    alloc.cleanup();

    let barray = Box::new_in([42u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 128); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
}

#[test]
fn single_page_alloc_exceeds_page_size() {
    let alloc = CQueueAllocatorInner::new();
    assert!(alloc.allocate(Layout::new::<[u8; 8000]>()).is_err())
    // let _ = Box::new_in([42u8; 8000], alloc.handle());
}

#[test]
fn single_page_list_alloc() {
    let alloc = CQueueAllocatorInner::new();
    let mut boxes = Vec::new();
    for i in 0..10 {
        boxes.push(Box::new_in([i as u8; 400], alloc.handle()))
    }

    // 4000 byte
    assert_eq!(alloc.dbg_alloc_total(), 4000);
    for i in 0..10 {
        assert_eq!(boxes[i][0], i as u8);
    }

    // Drop the last 2000 byte
    for _ in 0..5 {
        boxes.pop();
    }

    assert_eq!(alloc.dbg_alloc_total(), 2000);

    drop(boxes);

    assert!(alloc.dbg_is_empty());
}
