#![allow(unused)]

macro_rules! log_scope {
    () => {
        $crate::logger::Logger::end_scope()
    };
    ($i: expr) => {
        $crate::logger::Logger::begin_scope($i);
    };
    ($i: expr, $s: expr) => {
        $crate::logger::Logger::begin_scope(format!("{}: {}", $i, $s));
    };
    ($i: expr => { $e:expr }) => {{
        $crate::logger::Logger::begin_scope($i);
        let ret = $e;
        $crate::logger::Logger::end_scope();
        ret
    }};
}
