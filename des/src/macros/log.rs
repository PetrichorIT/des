#![allow(unused)]

macro_rules! log_scope {
    () => {
        $crate::logger::Logger::leave_scope()
    };
    ($i: expr) => {
        $crate::logger::Logger::enter_scope($i);
    };
    ($i: expr, $s: expr) => {
        $crate::logger::Logger::enter_scope(format!("{}: {}", $i, $s));
    };
    ($i: expr => { $e:expr }) => {{
        $crate::logger::Logger::enter_scope($i);
        let ret = $e;
        $crate::logger::Logger::leave_scope();
        ret
    }};
}
