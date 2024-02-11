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
#[allow(unused_allocation)]
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
fn test_collections() {
    let v = vec![1u8, 2, 3];
    assert_eq!(v.byte_len(), 3);

    let v = vec![String::new(), format!("Hello World"), format!("ABC")];
    assert_eq!(v.byte_len(), 11 + 3);
}
