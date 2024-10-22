#![allow(unused)]

use spin::Mutex;
use std::{io, sync::Arc};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug, Clone)]
pub struct MakeMockWriter {
    lines: Arc<Mutex<String>>,
}

#[derive(Debug, Clone)]
pub struct MockWriter {
    lines: Arc<Mutex<String>>,
}

impl io::Write for MockWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut lines = self.lines.lock();
        lines.push_str(&String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeMockWriter {
    pub fn new() -> Self {
        MakeMockWriter {
            lines: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn content(&self) -> String {
        self.lines.lock().clone()
    }
}

impl<'a> MakeWriter<'a> for MakeMockWriter {
    type Writer = MockWriter;
    fn make_writer(&'a self) -> Self::Writer {
        MockWriter {
            lines: self.lines.clone(),
        }
    }
}
