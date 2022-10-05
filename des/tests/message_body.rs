use des::prelude::*;

macro_rules! test_primive {
    ($ident:ident, $t:ty, $e:expr, $s:expr) => {
        #[test]
        fn $ident() {
            let value: $t = $e;
            assert_eq!(value.byte_len(), $s)
        }
    };
}
test_primive!(test_void, (), (), 0);

test_primive!(test_u8, u8, 0, 1);
test_primive!(test_u16, u16, 0, 2);
test_primive!(test_u32, u32, 0, 4);
test_primive!(test_u64, u64, 0, 8);
test_primive!(test_u128, u128, 0, 16);

test_primive!(test_i8, i8, 0, 1);
test_primive!(test_i16, i16, 0, 2);
test_primive!(test_i32, i32, 0, 4);
test_primive!(test_i64, i64, 0, 8);
test_primive!(test_i128, i128, 0, 16);

test_primive!(test_f32, f32, 0.0, 4);
test_primive!(test_f64, f64, 0.0, 8);

test_primive!(test_bool, bool, true, 1);
test_primive!(test_char, char, 'a', 4);

#[test]
fn test_string() {
    assert_eq!(String::new().byte_len(), 0);
    assert_eq!("Hello World".to_string().byte_len(), 11);
    assert_eq!("Hello World😀".to_string().byte_len(), 15);
}

#[test]
fn test_box() {
    assert_eq!(Box::new(0u8).byte_len(), 1);
    assert_eq!(Box::new(0i128).byte_len(), 16);
    assert_eq!(Box::new(String::from("Hello World")).byte_len(), 11);
    assert_eq!(Box::new(()).byte_len(), 0);
}

#[test]
fn test_option() {
    assert_eq!(Some(0u8).byte_len(), 1);
    let v: Option<u8> = None;
    assert_eq!(v.byte_len(), 0);

    assert_eq!(Some("Hello World".to_string()).byte_len(), 11);
    let v: Option<String> = None;
    assert_eq!(v.byte_len(), 0);
}

#[test]
fn test_result() {
    type R = Result<String, u8>;
    let v: R = Ok("Hello World".to_string());
    assert_eq!(v.byte_len(), 11);

    let v: R = Ok(String::new());
    assert_eq!(v.byte_len(), 0);

    let v: R = Err(0);
    assert_eq!(v.byte_len(), 1);

    let v: R = Err(16);
    assert_eq!(v.byte_len(), 1);
}

#[test]
fn test_cells() {
    use std::cell::*;

    let v = Cell::new(0u32);
    assert_eq!(v.byte_len(), 4);

    let v = Cell::new("Hello World".to_string());
    assert_eq!(v.byte_len(), 11);

    let v = RefCell::new(0u32);
    assert_eq!(v.byte_len(), 4);

    let v = RefCell::new("Hello World".to_string());
    assert_eq!(v.byte_len(), 11);

    let v = UnsafeCell::new(0u32);
    assert_eq!(v.byte_len(), 4);

    let v = UnsafeCell::new("Hello World".to_string());
    assert_eq!(v.byte_len(), 11);
}

#[test]
fn test_collections() {
    let v = vec![1u8, 2, 3];
    assert_eq!(v.byte_len(), 3);

    let v = vec![String::new(), format!("Hello World"), format!("ABC")];
    assert_eq!(v.byte_len(), 11 + 3);
}

// # Test macros

#[derive(MessageBody)]
struct A0;

#[derive(MessageBody)]
struct A1();

#[derive(MessageBody)]
struct A2 {}

#[test]
fn macro_struct_empty() {
    let v = A0;
    assert_eq!(v.byte_len(), 0);

    let v = A1();
    assert_eq!(v.byte_len(), 0);

    let v = A2 {};
    assert_eq!(v.byte_len(), 0)
}

#[derive(MessageBody)]
struct B0 {
    a: u8,
    b: char,
    c: Box<i128>,
}

#[derive(MessageBody)]
struct B1 {
    a: u32,
    b: String,
    c: Option<u8>,
}

#[test]
fn macro_struct_named() {
    let v = B0 {
        a: 0,
        b: 'b',
        c: Box::new(420),
    };
    assert_eq!(v.byte_len(), 21);

    let v = B1 {
        a: 0,
        b: "Hello World".to_string(),
        c: None,
    };
    assert_eq!(v.byte_len(), 15);

    let v = B1 {
        a: 0,
        b: "Hello World".to_string(),
        c: Some(1),
    };
    assert_eq!(v.byte_len(), 16);

    let v = B1 {
        a: 0,
        b: String::new(),
        c: None,
    };
    assert_eq!(v.byte_len(), 4);
}

#[derive(MessageBody)]
struct C0(u8, char, Box<i128>);

#[derive(MessageBody)]
struct C1(u32, String, Option<u8>);

#[test]
fn macro_struct_tupel() {
    let v = C0(0, 'a', Box::new(420));
    assert_eq!(v.byte_len(), 21);

    let v = C1(0, "Hello World".to_string(), None);
    assert_eq!(v.byte_len(), 15);

    let v = C1(0, "Hello World".to_string(), Some(11));
    assert_eq!(v.byte_len(), 16);

    let v = C1(0, String::new(), Some(11));
    assert_eq!(v.byte_len(), 5);
}

#[derive(MessageBody)]
enum D0 {
    Zero,
    TwentyOne(u8, char, Box<i128>),
    TwentyTwo { a: u16, b: char, c: Box<i128> },
    Dynamic(String),
}

#[test]
fn macro_enum() {
    let v = D0::Zero;
    assert_eq!(v.byte_len(), 0);

    let v = D0::TwentyOne(3, 'c', Box::new(0));
    assert_eq!(v.byte_len(), 21);

    let v = D0::TwentyTwo {
        a: 3,
        b: 'c',
        c: Box::new(0),
    };
    assert_eq!(v.byte_len(), 22);

    let v = D0::Dynamic("Hello World".to_string());
    assert_eq!(v.byte_len(), 11);

    let v = D0::Dynamic(String::new());
    assert_eq!(v.byte_len(), 0)
}
