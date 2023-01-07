#![allow(unused)]

macro_rules! log_scope {
    () => {
        $crate::runtime::Logger::end_scope()
    };
    ($i: expr) => {
        $crate::runtime::Logger::begin_scope($i);
    };
    ($i: expr, $s: expr) => {
        $crate::runtime::ScopedLogger::begin_scope(format!("{}: {}", $i, $s));
    };
    ($i: expr => { $e:expr }) => {{
        $crate::runtime::ScopedLogger::begin_scope($i);
        let ret = $e;
        $crate::runtime::ScopedLogger::end_scope();
        ret
    }};
}

// macro_rules! log_scope {
//     () => {
//         $crate::runtime::StandardLogger::end_scope();
//     };
//     ($i: expr) => {
//         $crate::runtime::StandardLogger::begin_scope($i);
//     };
//     ($i: expr, $s: expr) => {
//         $crate::runtime::StandardLogger::begin_scope_with_suffix($i, $s);
//     };
//     ($i: expr => { $e:expr }) => {{
//         $crate::runtime::StandardLogger::begin_scope($i);
//         let ret = $e;
//         $crate::runtime::StandardLogger::end_scope();
//         ret
//     }};
// }
