use cqueue::CQueue;
use std::time::Duration;

fn main() {
    let mut cqueue = CQueue::new(10, Duration::from_secs(1));
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
