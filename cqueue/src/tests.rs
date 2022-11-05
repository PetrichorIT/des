use super::*;
use rand::distributions::Uniform;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::*;
use std::time::Duration;

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

//     let dll = DLL::from_iter(events.clone());
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

//     let dll = DLL::from_iter(events.clone());
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

//     let dll = DLL::from_iter(events.clone());
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

//     let mut dll = DLL::from_iter(events.clone());

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

//     let dll = DLL::from(events);
//     let dll2 = DLL::from(events);

//     assert_eq!(dll, dll2)
// }

// #[test]
// fn linked_list_unordered_in_eq() {
//     let dll = DLL::from([
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//         (1, Duration::from_secs_f64(1.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         (2, Duration::from_secs_f64(2.0)),
//     ]);
//     let dll2 = DLL::from([
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
//     let dll = DLL::from([
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

//     let dll = DLL::from([
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

//     let dll = DLL::from([
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
//     let mut dll = DLL::from([
//         // (1, Duration::from_secs_f64(1.0)),
//         // (2, Duration::from_secs_f64(2.0)),
//         // (3, Duration::from_secs_f64(3.0)),
//         (4, Duration::from_secs_f64(4.0)),
//         (5, Duration::from_secs_f64(5.0)),
//         (6, Duration::from_secs_f64(6.0)),
//     ]);

//     let e1 = dll.add(1, Duration::from_secs_f64(1.0));
//     let e2 = dll.add(2, Duration::from_secs_f64(2.0));
//     let e3 = dll.add(3, Duration::from_secs_f64(3.0));

//     assert_eq!(dll.len(), 6);
//     e1.cancel();
//     assert_eq!(dll.len(), 5);
//     e3.cancel();
//     assert_eq!(dll.len(), 4);
//     e2.cancel();
//     assert_eq!(dll.len(), 3);

//     assert_eq!(
//         dll.into_iter().collect::<Vec<_>>(),
//         vec![
//             (4, Duration::from_secs_f64(4.0)),
//             (5, Duration::from_secs_f64(5.0)),
//             (6, Duration::from_secs_f64(6.0)),
//         ]
//     )
// }

// #[test]
// fn linked_list_remove_back() {
//     let mut dll = DLL::from([
//         (1, Duration::from_secs_f64(1.0)),
//         (2, Duration::from_secs_f64(2.0)),
//         (3, Duration::from_secs_f64(3.0)),
//         // (4, Duration::from_secs_f64(4.0)),
//         // (5, Duration::from_secs_f64(5.0)),
//         // (6, Duration::from_secs_f64(6.0)),
//     ]);

//     let e1 = dll.add(4, Duration::from_secs_f64(4.0));
//     let e2 = dll.add(5, Duration::from_secs_f64(5.0));
//     let e3 = dll.add(6, Duration::from_secs_f64(6.0));

//     assert_eq!(dll.len(), 6);
//     e3.cancel();
//     assert_eq!(dll.len(), 5);
//     e1.cancel();
//     assert_eq!(dll.len(), 4);
//     e2.cancel();
//     assert_eq!(dll.len(), 3);

//     assert_eq!(
//         dll.into_iter().collect::<Vec<_>>(),
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

#[test]
fn cqueue_zero_bucket_cancel() {
    let mut cqueue = CQueue::new(10, Duration::new(1, 0));
    let mut handles = (0..10)
        .map(|i| cqueue.add(Duration::ZERO, i))
        .collect::<Vec<_>>();
    assert_eq!(cqueue.len_zero(), 10);

    // remove element 6
    handles.remove(6).cancel();
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

    handles.remove(3).cancel();

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

    handles.remove(0).cancel();

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

#[test]
fn cqueue_out_of_order_with_cancel() {
    let mut cqueue = CQueue::new(32, Duration::new(1, 0));
    let mut delay = Duration::new(1, 0);
    let mut rng = SmallRng::seed_from_u64(123);
    let mut events = (0..200)
        .map(|v| {
            delay += Duration::from_secs_f64(rng.sample(Uniform::new(0.1, 1.0)));
            (v, delay, rng.sample(Uniform::new(1, 10)) == 8)
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
                h.cancel();
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
    assert_eq!(c, 199);
}

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

#[test]
fn cqueue_out_of_order_boxes_with_cancel() {
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
    let handles = event_boxes
        .into_iter()
        .map(|(t, from, n)| (from..(from + n)).map(move |i| (i, t)))
        .flatten()
        .map(|(e, t)| (cqueue.add(t, e), e, rng.sample(Uniform::new(1, 10)) == 2))
        .collect::<Vec<_>>();

    // Cancel events
    let cancelled = handles
        .into_iter()
        .filter_map(|(h, e, flg)| {
            if flg {
                h.cancel();
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
