use crate::time::{Duration, SimTime};

///
/// A timeline that compresses values, and is infinite
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlottedActivityTimeline {
    slot_size: Duration,
    last_slot: ActivityTimeline,

    datapoints: Vec<ActivityDatapoint>,
}

impl SlottedActivityTimeline {
    ///
    /// Creates a new instance of [Self].
    ///
    #[must_use]
    pub fn new(slot_size: Duration) -> Self {
        Self {
            slot_size,
            last_slot: ActivityTimeline::new(SimTime::now(), slot_size),

            datapoints: Vec::new(),
        }
    }

    ///
    /// Adds a record to the timeline.
    ///
    pub fn record_activity(&mut self, mut duration: Duration, magnitude: f64) {
        let mut time = SimTime::now();
        while !self.last_slot.write(&mut time, &mut duration, magnitude) {
            // (1) integrate last slot into globals dataset.
            // (2) Build new datapoint
            let mut new_datapoint =
                ActivityTimeline::new(self.last_slot.t0 + self.slot_size, self.slot_size);

            std::mem::swap(&mut new_datapoint, &mut self.last_slot);
            let datapoint = new_datapoint.into_datapoint();

            self.datapoints.push(datapoint);
        }
    }

    ///
    /// Removes cach inherence
    ///
    pub fn finish(&mut self) {
        let mut new_datapoint =
            ActivityTimeline::new(self.last_slot.t0 + self.slot_size, self.slot_size);

        std::mem::swap(&mut new_datapoint, &mut self.last_slot);
        let datapoint = new_datapoint.into_datapoint();

        self.datapoints.push(datapoint);
    }

    /// Outputs the results
    ///
    /// # Panics
    ///
    /// Panics if the collected datapoints are missformatted.
    ///
    pub fn print(&self) {
        for datapoint in &self.datapoints {
            assert_eq!(datapoint.duration, self.slot_size);
            println!("{}: {}", datapoint.time, datapoint.magnitude);
        }
    }
}

///
/// A activity timeline that has f64 activity states over time.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTimeline {
    t0: SimTime,
    dur: Duration,

    datapoints: Vec<ActivityDatapoint>,
}

impl ActivityTimeline {
    fn t1(&self) -> SimTime {
        self.t0 + self.dur
    }

    ///
    /// Creates a new instance of [Self].
    ///
    #[must_use]
    pub fn new(t0: SimTime, dur: Duration) -> Self {
        Self {
            t0,
            dur,
            datapoints: Vec::new(),
        }
    }

    ///
    /// Adds a record to the timeline, returning whether the datapoint could be added
    /// to the current timeline based on the timeline coverage.
    ///
    pub fn record_activity_at(
        &mut self,
        mut time: SimTime,
        mut duration: Duration,
        magnitude: f64,
    ) -> bool {
        self.write(&mut time, &mut duration, magnitude)
    }

    fn write(&mut self, time: &mut SimTime, duration: &mut Duration, magnitude: f64) -> bool {
        debug_assert!(*time >= self.t0, "We timetraveled again");
        // To far in the furture, assume to far in the past is not possible
        if *time >= self.t1() {
            return false;
        }

        if *time + *duration >= self.t1() {
            // Write partial ptr
            // Pic:
            //  | ... the slot ... | ... the next ... |
            //  t0                 t1
            //             time            time+duration
            let dur_doable = self.t1() - *time;
            *duration -= dur_doable;
            *time = self.t1();

            self.datapoints.push(ActivityDatapoint {
                time: *time,
                duration: dur_doable,
                magnitude,
            });

            false
        } else {
            // Write a datpoint fully contained within this set
            // (can have parts in prev sets, we dont care)
            self.datapoints.push(ActivityDatapoint {
                time: *time,
                duration: *duration,
                magnitude,
            });
            true
        }
    }

    fn into_datapoint(self) -> ActivityDatapoint {
        let total = self.dur.as_secs_f64();

        let acc = self
            .datapoints
            .into_iter()
            .fold(0.0, |acc, c| acc + (c.magnitude * c.duration.as_secs_f64()));

        ActivityDatapoint {
            time: self.t0,
            duration: self.dur,
            magnitude: acc / total,
        }
    }
}

///
/// A datapoint that describes an activity.
///
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityDatapoint {
    /// The point in time the activity began.
    pub time: SimTime,
    /// The duration that the activity occupied
    pub duration: Duration,
    /// The magnitude of the activity as a f64 (default 1.0)
    pub magnitude: f64,
}

impl Eq for ActivityDatapoint {}
