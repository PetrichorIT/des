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
