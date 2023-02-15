#[macro_export]
macro_rules! check_err {
    ($e:expr => $code:expr, $msg:literal) => {
        let e = $e.unwrap();
        assert_eq!(e.kind, $code);
        assert!(e.span.is_some());
        assert_eq!(format!("{}", e.internal), $msg);
        assert!(e.solution().is_none())
    };
    ($e:expr => $code:expr, $msg:literal, $solution:expr) => {
        let e = $e.unwrap();
        assert_eq!(e.kind, $code);
        assert!(e.span.is_some());
        assert_eq!(format!("{}", e.internal), $msg);
        assert_eq!(e.solution().unwrap().description, $solution);
    };
}
