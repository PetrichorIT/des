use std::alloc::Layout;
use std::mem::size_of;

use super::{boxed::*, *};
use rand::distr::Uniform;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::*;

#[test]
fn alloc_single_page_one_alloc_one_allocator() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let bu32 = LocalBox::new_in(42u32, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu32);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let bu64 = LocalBox::new_in(42u64, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu64);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let bu128 = LocalBox::new_in(42u128, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu128);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    // Now layout will grow

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let barray = LocalBox::new_in([42u8; 55], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 56); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
    drop(alloc);

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let barray = LocalBox::new_in([42u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 128); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
    drop(alloc);
}

#[test]
fn alloc_single_page_one_alloc_shared_allocator() {
    // Layout will allways be big enoght for a Free Node

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let bu32 = LocalBox::new_in(42u32, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu32);
    assert!(alloc.dbg_is_empty());

    let bu64 = LocalBox::new_in(42u64, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu64);
    assert!(alloc.dbg_is_empty());

    let bu128 = LocalBox::new_in(42u128, alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 16);
    drop(bu128);
    assert!(alloc.dbg_is_empty());

    // Now layout will grow

    let barray = LocalBox::new_in([42u8; 55], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 56); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());

    let barray = LocalBox::new_in([42u8; 128], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 128); // will be aligned for free node, with align 8
    drop(barray);
    assert!(alloc.dbg_is_empty());
}

#[test]
fn alloc_single_page_alloc_exceeds_page_size() {
    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    assert!(alloc
        .handle()
        .allocate(Layout::new::<[u8; 8000]>())
        .is_err())
    // let _ = Box::new_in([42u8; 8000], alloc.handle());
}

#[test]
fn alloc_single_page_list_alloc() {
    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let mut boxes = Vec::new();
    for i in 0..10 {
        boxes.push(LocalBox::new_in([i as u8; 400], alloc.handle()))
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

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let mut boxes = Vec::new();
    for _ in 0..100 {
        boxes.push(LocalBox::new_in(
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
    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let b1 = LocalBox::new_in([0u8; 2500], alloc.handle());
    assert_eq!(alloc.dbg_alloc_total(), 2504); // align
    assert_eq!(alloc.dbg_pages(), 1);

    // remaining bytes of page 1 were skipped
    // since elements are asssumed to be 2500 bytes big

    let b2 = LocalBox::new_in([0u8; 128], alloc.handle());
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

    let alloc = CQueueLLAllocatorInner::with_page_size(4096);
    let mut list = Vec::new();
    for _ in 1..10 {
        let b = LocalBox::new_in(Word::new(), alloc.handle());
        list.push(b)
    }
    alloc.info();

    list.remove(2);

    alloc.info();

    drop(list);

    alloc.info();

    for _ in 1..10 {
        let b = LocalBox::new_in(Word::new(), alloc.handle());
        std::mem::forget(b);
    }

    alloc.info();
}

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

#[test]
fn cqueue_zero_bucket_cancel() {
    let mut cqueue = CQueue::new(10, Duration::new(1, 0));
    let mut handles = (0..10)
        .map(|i| cqueue.add(Duration::ZERO, i))
        .collect::<Vec<_>>();
    assert_eq!(cqueue.len_zero(), 10);

    // remove element 6
    cqueue.cancel(handles.remove(6));
    assert_eq!(cqueue.len(), 9);
    assert_eq!(cqueue.len_zero(), 9);

    let mut c = 0;
    while !cqueue.is_empty() {
        if c == 6 {
            c += 1;
            continue;
        }
        let (e, _) = cqueue.fetch_next();
        assert_eq!(e, c);
        c += 1;
    }
    assert_eq!(c, 10);

    // STAGE 2 - without forwarding to the current event

    let mut handles = (0..10)
        .map(|i| cqueue.add(Duration::new(9, 0), i))
        .collect::<Vec<_>>();
    assert_eq!(cqueue.len_zero(), 0);

    cqueue.cancel(handles.remove(3));

    let mut c = 0;
    while !cqueue.is_empty() {
        if c == 3 {
            c += 1;
            continue;
        }
        let (e, _) = cqueue.fetch_next();
        assert_eq!(e, c);
        c += 1;
    }
    assert_eq!(c, 10);

    // STAGE 3: allready forwared

    let mut handles = (0..10)
        .map(|i| cqueue.add(Duration::new(9, 0), i))
        .collect::<Vec<_>>();
    assert_eq!(cqueue.len_zero(), 10);

    cqueue.cancel(handles.remove(0));

    let mut c = 1;
    while !cqueue.is_empty() {
        // if c == 0 { ... }
        assert_eq!(cqueue.len(), 10 - c);
        let (e, _) = cqueue.fetch_next();
        assert_eq!(e, c);
        c += 1;
    }
    assert_eq!(c, 10);
}

#[test]
fn cqueue_out_of_order_with_overlaps() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut delay = Duration::new(1, 0);
    let mut rng = SmallRng::seed_from_u64(123);
    let mut events = (0..200)
        .map(|v| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.1, 1.0).unwrap()));
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

#[test]
fn cqueue_out_of_order_with_cancel() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut delay = Duration::new(1, 0);
    let mut rng = SmallRng::seed_from_u64(123);
    let mut events = (0..200)
        .map(|v| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.1, 1.0).unwrap()));
            (v, delay, rng.sample(Uniform::new(1, 10).unwrap()) == 8)
        })
        .collect::<Vec<_>>();
    events.shuffle(&mut rng);
    let handles = events
        .into_iter()
        .map(|(event, time, cancel)| (cqueue.add(time, event), cancel, event))
        .collect::<Vec<_>>();

    let canceled = handles
        .into_iter()
        .filter_map(|(h, flg, event)| {
            if flg {
                cqueue.cancel(h);
                Some(event)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut c = 0;
    let mut last_time = Duration::ZERO;
    while !cqueue.is_empty() {
        if canceled.contains(&c) {
            c += 1;
            continue;
        }
        let (e, t) = cqueue.fetch_next();
        assert_eq!(e, c);
        assert!(t > last_time);

        last_time = t;
        c += 1;
    }
    // The 200th event was canceld thus the loop broke, event though one
    // iteration was still due (to be simpliar to previous test cases)
    assert_eq!(c, 200); // TODO: fix should be 199
}

#[test]
fn cqueue_out_of_order_boxes_overlapping() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut rng = SmallRng::seed_from_u64(123);
    let mut delay = Duration::new(1, 0);
    let mut s = 0;
    let mut event_boxes = (0..100)
        .map(|_| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.0, 1.0).unwrap()));
            let n = rng.sample(Uniform::new(1, 4).unwrap());
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

#[test]
fn cqueue_out_of_order_boxes_with_cancel() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut rng = SmallRng::seed_from_u64(123);
    let mut delay = Duration::new(1, 0);
    let mut s = 0;
    let mut event_boxes = (0..100)
        .map(|_| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.0, 1.0).unwrap()));
            let n = rng.sample(Uniform::new(1, 4).unwrap());
            let old_s = s;
            s += n;
            (delay, old_s, n)
        })
        // .map(|(t, from, n)| (from..(from + n)).map(|i| (i, t)))
        // .flatten()
        .collect::<Vec<_>>();

    event_boxes.shuffle(&mut rng);
    let handles = event_boxes
        .into_iter()
        .map(|(t, from, n)| (from..(from + n)).map(move |i| (i, t)))
        .flatten()
        .map(|(e, t)| {
            (
                cqueue.add(t, e),
                e,
                rng.sample(Uniform::new(1, 10).unwrap()) == 2,
            )
        })
        .collect::<Vec<_>>();

    // Cancel events
    let cancelled = handles
        .into_iter()
        .filter_map(|(h, e, flg)| {
            if flg {
                cqueue.cancel(h);
                Some(e)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut c = 0;
    let mut lt = Duration::ZERO;
    while !cqueue.is_empty() {
        if cancelled.contains(&c) {
            c += 1;
            continue;
        }
        let (e, t) = cqueue.fetch_next();
        assert_eq!(e, c);
        assert!(t >= lt);
        c += 1;
        lt = t;
    }
}

#[test]
fn cqueue_cancel_validity() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut handles = (0..10)
        .map(|i| Some(cqueue.add(Duration::from_secs(i), i)))
        .collect::<Vec<_>>();
    assert_eq!(cqueue.len(), 10);

    // Succesful cancel 0, 9
    cqueue.cancel(handles[0].take().unwrap());
    cqueue.cancel(handles[9].take().unwrap());

    assert_eq!(cqueue.len(), 8);

    // Cur [1,2,3,4,5,6,7,8]
    for i in 1..4 {
        let event = cqueue.fetch_next();
        assert_eq!(event.0, i);
    }
    assert_eq!(cqueue.len(), 5);

    // Cur [4,5,6,7,8]
    cqueue.cancel(handles[2].take().unwrap());
    assert_eq!(cqueue.len(), 5);

    cqueue.cancel(handles[3].take().unwrap());
    assert_eq!(cqueue.len(), 5);

    // Suc again
    cqueue.cancel(handles[8].take().unwrap());
    assert_eq!(cqueue.len(), 4);

    // Cur [4,5,6,7]
    let mut c = 0;
    while !cqueue.is_empty() {
        let _ = cqueue.fetch_next();
        c += 1;
    }

    assert_eq!(c, 4)
}

#[test]
fn cqueue_cancel_validity_2() {
    let mut cqueue = CQueue::new(10, Duration::new(3, 0));
    let mut handles = (0..10)
        .map(|i| Some(cqueue.add(Duration::from_secs(i), i)))
        .collect::<Vec<_>>();
    assert_eq!(cqueue.len(), 10);

    // Succesful cancel 0, 9
    cqueue.cancel(handles[0].take().unwrap());
    cqueue.cancel(handles[9].take().unwrap());

    assert_eq!(cqueue.len(), 8);

    // Cur [1,2,3,4,5,6,7,8]
    for i in 1..4 {
        let event = cqueue.fetch_next();
        assert_eq!(event.0, i);
    }
    assert_eq!(cqueue.len(), 5);

    // Cur [4,5,6,7,8]
    cqueue.cancel(handles[2].take().unwrap());
    assert_eq!(cqueue.len(), 5);

    cqueue.cancel(handles[3].take().unwrap());
    assert_eq!(cqueue.len(), 5);

    // Suc again
    cqueue.cancel(handles[8].take().unwrap());
    assert_eq!(cqueue.len(), 4);

    // Cur [4,5,6,7]
    let mut c = 0;
    while !cqueue.is_empty() {
        let _ = cqueue.fetch_next();
        c += 1;
    }

    assert_eq!(c, 4)
}
