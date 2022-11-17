// use super::linked_list::*;
// use super::clinked_list::*;
use super::*;
use rand::distributions::Uniform;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::*;
use std::alloc::{Allocator, Layout};
use std::mem::size_of;
use std::time::Duration;

#[test]
fn alloc_single_page_one_alloc_one_allocator() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueLLAllocatorInner::new();
    let bu32 = Box::new_in(42u32, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu32);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueLLAllocatorInner::new();
    let bu64 = Box::new_in(42u64, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu64);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueLLAllocatorInner::new();
    let bu128 = Box::new_in(42u128, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu128);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    // Now layout will grow

    let alloc = CQueueLLAllocatorInner::new();
    let barray = Box::new_in([42u8; 55], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 56); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueLLAllocatorInner::new();
    let barray = Box::new_in([42u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 128); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
    drop(alloc);
}

#[test]
fn alloc_single_page_one_alloc_shared_allocator() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueLLAllocatorInner::new();
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
fn alloc_single_page_alloc_exceeds_page_size() {
    let alloc = CQueueLLAllocatorInner::new();
    assert!(alloc
        .handle()
        .allocate(Layout::new::<[u8; 8000]>())
        .is_err())
    // let _ = Box::new_in([42u8; 8000], alloc.handle());
}

#[test]
fn alloc_single_page_list_alloc() {
    let alloc = CQueueLLAllocatorInner::new();
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

#[test]
fn alloc_multiple_pages_same_size_allocation() {
    #[allow(dead_code)]
    struct A {
        bytes: [u8; 32],
        int: u128,
        s: String,
    }

    let alloc = CQueueLLAllocatorInner::new();
    let mut boxes = Vec::new();
    for _ in 0..100 {
        boxes.push(Box::new_in(
            A {
                bytes: [0; 32],
                int: 42,
                s: String::from("Hallow str"),
            },
            alloc.handle(),
        ));
    }

    assert!(
        alloc.dbg_alloc_total() >= size_of::<A>() * 100,
        "alloc: {} expected: {} * 100",
        alloc.dbg_alloc_total(),
        size_of::<A>()
    )
}

#[test]
fn alloc_multiple_pages_skip_to_small_elements() {
    let alloc = CQueueLLAllocatorInner::new();
    let b1 = Box::new_in([0u8; 2500], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 2504); // align
    assert_eq!(alloc.dbg_pages(), 1);

    // remaining bytes of page 1 were skipped
    // since elements are asssumed to be 2500 bytes big

    let b2 = Box::new_in([0u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 2504 + 128); // align
    assert_eq!(alloc.dbg_pages(), 2);

    drop(b1);
    drop(b2);

    assert_eq!(alloc.dbg_alloc_total(), 0);
    assert_eq!(alloc.dbg_pages(), 2);

    drop(alloc);
}

#[test]
fn alloc_16_byteboxes() {
    struct Word {
        _opaque: [u8; 16],
    }

    impl Word {
        fn new() -> Self {
            Self { _opaque: [42; 16] }
        }
    }

    assert_eq!(std::mem::size_of::<Word>(), 16);

    let alloc = CQueueLLAllocatorInner::new();
    let mut list = Vec::new();
    for _ in 1..10 {
        let b = Box::new_in(Word::new(), alloc.handle());
        list.push(b)
    }
    alloc.info();

    list.remove(2);

    alloc.info();

    drop(list);

    alloc.info();

    for _ in 1..10 {
        let b = Box::new_in(Word::new(), alloc.handle());
        std::mem::forget(b);
    }

    alloc.info();
}

// #[test]
// fn clinked_list() {
//     let mut ls = CacheOptimizedLinkedList::with_capacity(4);
//     ls.add(1, Duration::from_secs(1), 1);
//     println!("{:?}", ls);
//     ls.add(2, Duration::from_secs(2), 2);
//     println!("{:?}", ls);
//     ls.add(3, Duration::from_secs(3), 3);
//     println!("{:?}", ls);

//     while let Some((event, time, _)) = ls.pop_min() {
//         println!("popped {} at {:?}", event, time);
//         println!("{:?}", ls)
//     }

//     ls.add(4, Duration::from_secs(4), 4);
//     println!("{:?}", ls);
// }

// #[test]
// fn linked_list_ordered_in_ordered_out() {
//     let events = [
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ];

//     let dll = DLL::from_iter(events.clone());
//     let event = dll.into_iter().collect::<Vec<_>>();

//     assert_eq!(&events[..], &event);
// }

// #[test]
// fn linked_list_unordered_in_ordered_out() {
//     let mut events = [
//         (5, Duration::from_secs_f64(5.0)),
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (6, Duration::from_secs_f64(6.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (3, Duration::from_secs_f64(3.0)),
//     ];

//     let dll = CacheOptimizedLinkedList::from_iter(events.clone());
//     let event = dll.into_iter().collect::<Vec<_>>();

//     events.sort();

//     assert_eq!(&events[..], &event);
// }

// #[test]
// fn linked_list_ordered_collision_in_retain() {
//     let events = [
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (3, Duration::from_secs_f64(4.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ];

//     let dll = CacheOptimizedLinkedList::from_iter(events.clone());
//     let event = dll.into_iter().collect::<Vec<_>>();

//     assert_eq!(&events[..], &event);
// }

// #[test]
// fn linked_list_unordered_collision_in_retain() {
//     let mut events = [
//         (5, Duration::from_secs_f64(5.0)),
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (6, Duration::from_secs_f64(6.0)),
//         (4, Duration::from_secs_f64(1.0)),
//         (3, Duration::from_secs_f64(3.0)),
//     ];

//     let dll = CacheOptimizedLinkedList::from_iter(events.clone());
//     let event = dll.into_iter().collect::<Vec<_>>();

//     events.sort_by(|l, r| l.1.cmp(&r.1));

//     assert_eq!(&events[..], &event);
// }

// #[test]
// fn linked_list_iter_and_iter_mut() {
//     let events = [
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ];

//     let mut dll = CacheOptimizedLinkedList::from_iter(events.clone());

//     let mut c = 1;
//     for item in dll.iter() {
//         assert_eq!(*item.0, c);
//         assert_eq!(item.1.as_secs(), c);
//         c += 1;
//     }

//     for item in dll.iter_mut() {
//         *item.0 += 1;
//     }

//     let mut c = 1;
//     for item in dll.iter() {
//         assert_eq!(*item.0, c + 1);
//         assert_eq!(item.1.as_secs(), c);
//         c += 1;
//     }
// }

// #[test]
// fn linked_list_ordered_in_eq() {
//     let events = [
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ];

//     let dll = CacheOptimizedLinkedList::from(events);
//     let dll2 = CacheOptimizedLinkedList::from(events);

//     assert_eq!(dll, dll2)
// }

// #[test]
// fn linked_list_unordered_in_eq() {
//     let dll = CacheOptimizedLinkedList::from([
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//         (1, Duration::from_secs_f64(1.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (2, Duration::from_secs_f64(2.0)),
//     ]);

//     let dll2 = CacheOptimizedLinkedList::from([
//         (5, Duration::from_secs_f64(5.0)),
//         (1, Duration::from_secs_f64(1.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ]);

//     assert_eq!(dll, dll2)
// }

// #[test]
// fn linked_list_same_time_in_order() {
//     let dll = CacheOptimizedLinkedList::from([
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(3.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(3.0)),
//         (5, Duration::from_secs_f64(3.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ]);
//     let mut c = 1;
//     for item in dll {
//         assert_eq!(item.0, c);
//         c += 1;
//     }
//     assert_eq!(c, 7);

//     let dll = CacheOptimizedLinkedList::from([
//         // (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(3.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(3.0)),
//         (5, Duration::from_secs_f64(3.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ]);
//     let mut c = 2;
//     for item in dll {
//         assert_eq!(item.0, c);
//         c += 1;
//     }
//     assert_eq!(c, 7);

//     let dll = CacheOptimizedLinkedList::from([
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(3.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(3.0)),
//         (5, Duration::from_secs_f64(3.0)),
//         // (6, Duration::from_secs_f64(6.0)),
//     ]);
//     let mut c = 1;
//     for item in dll {
//         assert_eq!(item.0, c);
//         c += 1;
//     }
//     assert_eq!(c, 6);
// }

// #[test]
// fn linked_list_remove_min() {
//     let mut CacheOptimizedLinkedList = CacheOptimizedLinkedList::from([
//         // (1, Duration::from_secs_f64(1.0)),
//         // (2, Duration::from_secs_f64(2.0)),
//         // (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ]);

//     let e1 = CacheOptimizedLinkedList.add(1, Duration::from_secs_f64(1.0));
//     let e2 = CacheOptimizedLinkedList.add(2, Duration::from_secs_f64(2.0));
//     let e3 = CacheOptimizedLinkedList.add(3, Duration::from_secs_f64(3.0));

//     assert_eq!(CacheOptimizedLinkedList.len(), 6);
//     e1.cancel();
//     assert_eq!(CacheOptimizedLinkedList.len(), 5);
//     e3.cancel();
//     assert_eq!(CacheOptimizedLinkedList.len(), 4);
//     e2.cancel();
//     assert_eq!(CacheOptimizedLinkedList.len(), 3);

//     assert_eq!(
//         CacheOptimizedLinkedList.into_iter().collect::<Vec<_>>(),
//         vec![
//             (4, Duration::from_secs_f64(4.0)),
//             (5, Duration::from_secs_f64(5.0)),
//             (6, Duration::from_secs_f64(6.0)),
//         ]
//     )
// }

// #[test]
// fn linked_list_remove_back() {
//     let mut CacheOptimizedLinkedList = CacheOptimizedLinkedList::from([
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         // (4, Duration::from_secs_f64(4.0)),
//         // (5, Duration::from_secs_f64(5.0)),
//         // (6, Duration::from_secs_f64(6.0)),
//     ]);

//     let e1 = CacheOptimizedLinkedList.add(4, Duration::from_secs_f64(4.0));
//     let e2 = CacheOptimizedLinkedList.add(5, Duration::from_secs_f64(5.0));
//     let e3 = CacheOptimizedLinkedList.add(6, Duration::from_secs_f64(6.0));

//     assert_eq!(CacheOptimizedLinkedList.len(), 6);
//     e3.cancel();
//     assert_eq!(CacheOptimizedLinkedList.len(), 5);
//     e1.cancel();
//     assert_eq!(CacheOptimizedLinkedList.len(), 4);
//     e2.cancel();
//     assert_eq!(CacheOptimizedLinkedList.len(), 3);

//     assert_eq!(
//         CacheOptimizedLinkedList.into_iter().collect::<Vec<_>>(),
//         vec![
//             (1, Duration::from_secs_f64(1.0)),
//             (2, Duration::from_secs_f64(2.0)),
//             (3, Duration::from_secs_f64(3.0)),
//         ]
//     )
// }

#[test]
fn cqueue_simple_event_order_nonoverlapping() {
    let mut cqueue = CQueue::new(100, Duration::from_secs(1));
    let events = (0..=100).map(|e| (e, Duration::from_secs(e)));
    for (event, time) in events {
        cqueue.add(time, event);
    }

    assert_eq!(cqueue.len(), 101);

    let mut c = 0;
    while !cqueue.is_empty() {
        println!("Itr: {}", c);
        let (event, time) = cqueue.fetch_next();
        assert_eq!(c, event);
        assert_eq!(time.as_secs(), c);
        c += 1;
    }
    assert_eq!(c, 101);
    assert_eq!(cqueue.len(), 0);
}

#[test]
fn cqueue_simple_event_order_wrap_around() {
    // This test is identicall to 'cqueue_simple_event_order_nonoverlapping'
    // but with other config options for the cqueue.
    let mut cqueue = CQueue::new(20, Duration::from_secs(1));
    let events = (0..=100).map(|e| (e, Duration::from_secs(e)));
    for (event, time) in events {
        cqueue.add(time, event);
    }

    let mut c = 0;
    while !cqueue.is_empty() {
        let (event, time) = cqueue.fetch_next();
        assert_eq!(c, event);
        assert_eq!(time.as_secs(), c);
        c += 1;
    }
    assert_eq!(c, 101);
    assert_eq!(cqueue.len(), 0);
}

#[test]
fn cqueue_simple_event_out_of_order_nonoverlapping() {
    let mut cqueue = CQueue::new(100, Duration::from_secs(1));
    let mut events = (0..=100)
        .map(|e| (e, Duration::from_secs(e)))
        .collect::<Vec<_>>();
    let mut rng = SmallRng::seed_from_u64(123);
    events.shuffle(&mut rng);
    for (event, time) in events {
        cqueue.add(time, event);
    }

    assert_eq!(cqueue.len(), 101);

    let mut c = 0;
    while !cqueue.is_empty() {
        println!("Itr: {}", c);
        let (event, time) = cqueue.fetch_next();
        assert_eq!(c, event);
        assert_eq!(time.as_secs(), c);
        c += 1;
    }
    assert_eq!(c, 101);
    assert_eq!(cqueue.len(), 0);
}

#[test]
fn cqueue_simple_event_out_of_order_wrap_around() {
    // This test is identicall to 'cqueue_simple_event_order_nonoverlapping'
    // but with other config options for the cqueue.
    let mut cqueue = CQueue::new(20, Duration::from_secs(1));
    let mut events = (0..=100)
        .map(|e| (e, Duration::from_secs(e)))
        .collect::<Vec<_>>();
    let mut rng = SmallRng::seed_from_u64(123);
    events.shuffle(&mut rng);
    for (event, time) in events {
        cqueue.add(time, event);
    }

    let mut c = 0;
    while !cqueue.is_empty() {
        let (event, time) = cqueue.fetch_next();
        assert_eq!(c, event);
        assert_eq!(time.as_secs(), c);
        c += 1;
    }
    assert_eq!(c, 101);
    assert_eq!(cqueue.len(), 0);
}

#[test]
fn cqueue_zero_bucket_in_out() {
    let mut cqueue = CQueue::new(10, Duration::new(1, 0));
    for i in 0..10 {
        cqueue.add(Duration::ZERO, i);
    }
    assert_eq!(cqueue.len_zero(), 10);

    let mut c = 0;
    while !cqueue.is_empty() {
        assert_eq!(cqueue.len(), 10 - c);
        let (e, _) = cqueue.fetch_next();
        assert_eq!(e, c);
        c += 1;
    }
    assert_eq!(c, 10);

    // STAGE 2 - without forwarding to the current event

    for i in 0..10 {
        cqueue.add(Duration::new(9, 0), i);
    }
    assert_eq!(cqueue.len_zero(), 0);

    let mut c = 0;
    while !cqueue.is_empty() {
        assert_eq!(cqueue.len(), 10 - c);
        let (e, _) = cqueue.fetch_next();
        assert_eq!(e, c);
        c += 1;
    }
    assert_eq!(c, 10);

    // STAGE 3: allready forwared

    for i in 0..10 {
        cqueue.add(Duration::new(9, 0), i);
    }
    assert_eq!(cqueue.len_zero(), 10);

    let mut c = 0;
    while !cqueue.is_empty() {
        assert_eq!(cqueue.len(), 10 - c);
        let (e, _) = cqueue.fetch_next();
        assert_eq!(e, c);
        c += 1;
    }
    assert_eq!(c, 10);
}

// #[test]
// fn cqueue_zero_bucket_cancel() {
//     let mut cqueue = CQueue::new(10, Duration::new(1, 0));
//     let mut handles = (0..10)
//         .map(|i| cqueue.add(Duration::ZERO, i))
//         .collect::<Vec<_>>();
//     assert_eq!(cqueue.len_zero(), 10);

//     // remove element 6
//     handles.remove(6).cancel();
//     assert_eq!(cqueue.len(), 9);
//     assert_eq!(cqueue.len_zero(), 9);

//     let mut c = 0;
//     while !cqueue.is_empty() {
//         if c == 6 {
//             c += 1;
//             continue;
//         }
//         let (e, _) = cqueue.fetch_next();
//         assert_eq!(e, c);
//         c += 1;
//     }
//     assert_eq!(c, 10);

//     // STAGE 2 - without forwarding to the current event

//     let mut handles = (0..10)
//         .map(|i| cqueue.add(Duration::new(9, 0), i))
//         .collect::<Vec<_>>();
//     assert_eq!(cqueue.len_zero(), 0);

//     handles.remove(3).cancel();

//     let mut c = 0;
//     while !cqueue.is_empty() {
//         if c == 3 {
//             c += 1;
//             continue;
//         }
//         let (e, _) = cqueue.fetch_next();
//         assert_eq!(e, c);
//         c += 1;
//     }
//     assert_eq!(c, 10);

//     // STAGE 3: allready forwared

//     let mut handles = (0..10)
//         .map(|i| cqueue.add(Duration::new(9, 0), i))
//         .collect::<Vec<_>>();
//     assert_eq!(cqueue.len_zero(), 10);

//     handles.remove(0).cancel();

//     let mut c = 1;
//     while !cqueue.is_empty() {
//         // if c == 0 { ... }
//         assert_eq!(cqueue.len(), 10 - c);
//         let (e, _) = cqueue.fetch_next();
//         assert_eq!(e, c);
//         c += 1;
//     }
//     assert_eq!(c, 10);
// }

#[test]
fn cqueue_out_of_order_with_overlaps() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut delay = Duration::new(1, 0);
    let mut rng = SmallRng::seed_from_u64(123);
    let mut events = (0..200)
        .map(|v| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.1, 1.0)));
            (v, delay)
        })
        .collect::<Vec<_>>();
    events.shuffle(&mut rng);
    let _ = events
        .into_iter()
        .map(|(event, time)| cqueue.add(time, event))
        .collect::<Vec<_>>();

    let mut c = 0;
    let mut last_time = Duration::ZERO;
    while !cqueue.is_empty() {
        assert_eq!(cqueue.len(), 200 - c);
        let (e, t) = cqueue.fetch_next();
        assert_eq!(e, c);
        assert!(t > last_time);

        last_time = t;
        c += 1;
    }
    assert_eq!(c, 200);
}

// #[test]
// fn cqueue_out_of_order_with_cancel() {
//     let mut cqueue = CQueue::new(32, Duration::new(1, 0));
//     let mut delay = Duration::new(1, 0);
//     let mut rng = SmallRng::seed_from_u64(123);
//     let mut events = (0..200)
//         .map(|v| {
//             delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.1, 1.0)));
//             (v, delay, rng.sample(Uniform::new(1, 10)) == 8)
//         })
//         .collect::<Vec<_>>();
//     events.shuffle(&mut rng);
//     let handles = events
//         .into_iter()
//         .map(|(event, time, cancel)| (cqueue.add(time, event), cancel, event))
//         .collect::<Vec<_>>();

//     let canceled = handles
//         .into_iter()
//         .filter_map(|(h, flg, event)| {
//             if flg {
//                 h.cancel();
//                 Some(event)
//             } else {
//                 None
//             }
//         })
//         .collect::<Vec<_>>();

//     let mut c = 0;
//     let mut last_time = Duration::ZERO;
//     while !cqueue.is_empty() {
//         if canceled.contains(&c) {
//             c += 1;
//             continue;
//         }
//         let (e, t) = cqueue.fetch_next();
//         assert_eq!(e, c);
//         assert!(t > last_time);

//         last_time = t;
//         c += 1;
//     }
//     // The 200th event was canceld thus the loop broke, event though one
//     // iteration was still due (to be simpliar to previous test cases)
//     assert_eq!(c, 199);
// }

#[test]
fn cqueue_out_of_order_boxes_overlapping() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut rng = SmallRng::seed_from_u64(123);
    let mut delay = Duration::new(1, 0);
    let mut s = 0;
    let mut event_boxes = (0..100)
        .map(|_| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.0, 1.0)));
            let n = rng.sample(Uniform::new(1, 4));
            let old_s = s;
            s += n;
            (delay, old_s, n)
        })
        // .map(|(t, from, n)| (from..(from + n)).map(|i| (i, t)))
        // .flatten()
        .collect::<Vec<_>>();

    event_boxes.shuffle(&mut rng);
    let _ = event_boxes
        .into_iter()
        .map(|(t, from, n)| (from..(from + n)).map(move |i| (i, t)))
        .flatten()
        .map(|(e, t)| cqueue.add(t, e))
        .collect::<Vec<_>>();

    let mut c = 0;
    let mut lt = Duration::ZERO;
    while !cqueue.is_empty() {
        let (e, t) = cqueue.fetch_next();
        assert_eq!(e, c);
        assert!(t >= lt);
        c += 1;
        lt = t;
    }
}

// #[test]
// fn cqueue_out_of_order_boxes_with_cancel() {
//     let mut cqueue = CQueue::new(32, Duration::new(1, 0));
//     let mut rng = SmallRng::seed_from_u64(123);
//     let mut delay = Duration::new(1, 0);
//     let mut s = 0;
//     let mut event_boxes = (0..100)
//         .map(|_| {
//             delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.0, 1.0)));
//             let n = rng.sample(Uniform::new(1, 4));
//             let old_s = s;
//             s += n;
//             (delay, old_s, n)
//         })
//         // .map(|(t, from, n)| (from..(from + n)).map(|i| (i, t)))
//         // .flatten()
//         .collect::<Vec<_>>();

//     event_boxes.shuffle(&mut rng);
//     let handles = event_boxes
//         .into_iter()
//         .map(|(t, from, n)| (from..(from + n)).map(move |i| (i, t)))
//         .flatten()
//         .map(|(e, t)| (cqueue.add(t, e), e, rng.sample(Uniform::new(1, 10)) == 2))
//         .collect::<Vec<_>>();

//     // Cancel events
//     let cancelled = handles
//         .into_iter()
//         .filter_map(|(h, e, flg)| {
//             if flg {
//                 h.cancel();
//                 Some(e)
//             } else {
//                 None
//             }
//         })
//         .collect::<Vec<_>>();

//     let mut c = 0;
//     let mut lt = Duration::ZERO;
//     while !cqueue.is_empty() {
//         if cancelled.contains(&c) {
//             c += 1;
//             continue;
//         }
//         let (e, t) = cqueue.fetch_next();
//         assert_eq!(e, c);
//         assert!(t >= lt);
//         c += 1;
//         lt = t;
//     }
// }
