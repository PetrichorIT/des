#[test]
fn interning_test() {
    use super::*;

    let interner = Interner::new();

    let string_untyped = interner.intern(String::from("Hello world"));
    let string_typed = interner.intern_typed(String::from("DES"));

    let sized = interner.intern(12_u32);
    let boxed = interner.intern_boxed(Box::new(42_f64));

    assert_eq!(
        *string_untyped.cast::<String>().deref(),
        String::from("Hello world")
    );

    assert_eq!(*string_typed.deref(), String::from("DES"));

    assert_eq!(*sized.cast::<u32>(), 12);

    assert_eq!(*boxed.cast::<f64>().deref(), 42_f64);
}

#[test]
#[should_panic]
fn interning_test_panic_cast() {
    use super::*;

    let interner = Interner::new();

    let string_untyped = interner.intern(String::from("Hello world"));
    let _typed = string_untyped.cast::<usize>();
}
