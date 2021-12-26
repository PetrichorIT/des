pub mod bench {

    use std::{
        io::Write,
        path::PathBuf,
        time::{Duration, Instant},
    };

    ///
    /// Prevents the compiler from optimizing the a call syntax.
    ///
    pub fn black_box<T>(dummy: T) -> T {
        unsafe {
            let ret = std::ptr::read_volatile(&dummy);
            std::mem::forget(dummy);
            ret
        }
    }

    pub fn bench<T>(ctx: &mut BenchmarkCtx, name: &'static str, mut routine: T)
    where
        T: FnMut() -> (),
    {
        let mut benchmark = Benchmark::new(name);
        for _ in 0..ctx.itr {
            let timer = Timer::new(&mut benchmark);
            (routine)();
            timer.finish()
        }

        ctx.benchmarks.push(benchmark)
    }

    use serde_derive::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct BenchmarkCtx {
        name: String,
        itr: usize,
        benchmarks: Vec<Benchmark>,
    }

    impl BenchmarkCtx {
        pub fn new(name: &str, itr: usize) -> Self {
            Self {
                name: name.into(),
                itr,
                benchmarks: Vec::new(),
            }
        }

        pub fn load(name: &str) -> std::io::Result<Self> {
            let raw = std::fs::read_to_string(format!("benches/results/{}.yaml", name))?;
            let val: Self = serde_json::from_str(&raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            Ok(val)
        }

        pub fn finish(self, write_to_file: bool) -> std::io::Result<()> {
            use termcolor::*;

            let mut stream = StandardStream::stdout(ColorChoice::Always);
            let stream = &mut stream;

            stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            writeln!(stream, "Benchmark suit '{}' finished", self.name)?;
            writeln!(
                stream,
                "{}",
                std::iter::repeat("=")
                    .take(26 + self.name.len())
                    .collect::<String>()
            )?;
            writeln!(stream)?;

            match Self::load(&self.name) {
                Ok(old) => {
                    for bench in &self.benchmarks {
                        let old_bench = old.benchmarks.iter().find(|m| m.name == bench.name);
                        if let Some(old_bench) = old_bench {
                            stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                            writeln!(stream, "        Results of '{}':", bench.name)?;
                            stream.set_color(ColorSpec::new().set_bold(true))?;
                            write!(
                                stream,
                                "            avg = {}μs ({} samples)",
                                bench.avg.as_micros(),
                                bench.len,
                            )?;

                            let (avg_diff, new_faster) = match old_bench.avg.checked_sub(bench.avg)
                            {
                                Some(dur) => (dur, true),
                                None => (bench.avg - old_bench.avg, false),
                            };

                            let avg_diff_prec =
                                avg_diff.as_secs_f64() / old_bench.avg.as_secs_f64();
                            let avg_diff_prec = avg_diff_prec * 100.0;

                            if new_faster {
                                stream.set_color(
                                    ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true),
                                )?;
                                writeln!(stream, " {:.2}% faster", avg_diff_prec)?;
                            } else {
                                stream.set_color(
                                    ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true),
                                )?;
                                writeln!(stream, " {:.2}% slower", avg_diff_prec)?;
                            }

                            stream.set_color(ColorSpec::new().set_bold(true))?;
                            writeln!(stream, "            min = {}μs", bench.min.as_micros())?;
                            writeln!(stream, "            max = {}μs", bench.max.as_micros())?;
                        } else {
                            stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                            writeln!(stream, "        Results of '{}':", bench.name)?;
                            stream.set_color(ColorSpec::new().set_bold(true))?;
                            writeln!(
                                stream,
                                "            avg = {}μs ({} samples)",
                                bench.avg.as_micros(),
                                bench.len
                            )?;
                            writeln!(stream, "            min = {}μs", bench.min.as_micros())?;
                            writeln!(stream, "            max = {}μs", bench.max.as_micros())?;
                        }
                    }
                }
                Err(_e) => {
                    // DO NOTHING
                    for bench in &self.benchmarks {
                        stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                        writeln!(stream, "        Results of '{}':", bench.name)?;
                        stream.set_color(ColorSpec::new().set_bold(true))?;
                        writeln!(
                            stream,
                            "            avg = {}μs ({} samples)",
                            bench.avg.as_micros(),
                            bench.len
                        )?;
                        writeln!(stream, "            min = {}μs", bench.min.as_micros())?;
                        writeln!(stream, "            max = {}μs", bench.max.as_micros())?;
                    }
                }
            }

            stream.reset()?;
            writeln!(stream)?;

            if write_to_file {
                let str = serde_json::to_string(&self).unwrap();
                let path = PathBuf::from(format!("benches/results/{}.yaml", self.name));
                let prefix = path.parent().unwrap();
                std::fs::create_dir_all(prefix)?;

                let mut file = std::fs::File::create(path)?;

                write!(file, "{}", str).expect("Failed to write to file");
            }

            Ok(())
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Benchmark {
        name: String,
        min: Duration,
        max: Duration,
        avg: Duration,
        len: u32,
    }

    impl Benchmark {
        pub fn new(name: &str) -> Self {
            Self {
                name: name.into(),

                min: Duration::from_secs(u64::MAX),
                max: Duration::new(0, 0),
                avg: Duration::new(0, 0),
                len: 0,
            }
        }

        pub fn record(&mut self, messurement: Duration) {
            if messurement < self.min {
                self.min = messurement
            }

            if messurement > self.max {
                self.max = messurement
            }

            let avg = self.avg * self.len;
            let avg = avg + messurement;
            let avg = avg / (self.len + 1);
            self.avg = avg;

            self.len += 1;
        }
    }

    pub struct Timer<'a> {
        benchmark: &'a mut Benchmark,
        start: Instant,
    }

    impl<'a> Timer<'a> {
        pub fn new(benchmark: &'a mut Benchmark) -> Self {
            Self {
                benchmark,
                start: Instant::now(),
            }
        }

        fn finish(self) {
            let end = std::time::Instant::now();
            let dur = end - self.start;
            self.benchmark.record(dur);
        }
    }
}

pub struct ScopeTimer {
    name: &'static str,
    start: std::time::Instant,
}

impl ScopeTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }
}

impl Drop for ScopeTimer {
    fn drop(&mut self) {
        let end = std::time::Instant::now();
        let dur = end - self.start;

        println!("[ScopedTimer] {} took {}μs", self.name, dur.as_micros());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
