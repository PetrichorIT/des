use quote::quote;
use syn::{parse2, DeriveInput};

#[test]
fn struct_unit() {
    let input = quote! {
        struct Input;
    };

    let Ok(DeriveInput { ident, data, generics, .. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl ::des::net::message::MessageBody for Input {
                fn byte_len(&self) -> usize {
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_named() {
    let input = quote! {
        struct Input {
            a: u32,
            b: Vec<u8>,
            c: ()
        }
    };

    let Ok(DeriveInput { ident, data, generics, .. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl ::des::net::message::MessageBody for Input {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.a) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.b) +
                    <() as ::des::net::message::MessageBody>::byte_len(&self.c) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_named_generic_nonbounded() {
    let input = quote! {
        struct Input<T> {
            a: u32,
            b: Vec<u8>,
            c: T
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.a) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.b) +
                    <T as ::des::net::message::MessageBody>::byte_len(&self.c) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_named_generic_bounded() {
    let input = quote! {
        struct Input<T: Copy + Eq> {
            a: u32,
            b: Vec<u8>,
            c: T
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: Copy + Eq + ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.a) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.b) +
                    <T as ::des::net::message::MessageBody>::byte_len(&self.c) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_named_generic_where_clause() {
    let input = quote! {
        struct Input<T> where T: Copy + std::hash::Hash {
            a: u32,
            b: Vec<u8>,
            c: T
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T>
            where T: Copy + std::hash::Hash {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.a) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.b) +
                    <T as ::des::net::message::MessageBody>::byte_len(&self.c) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_unnamed() {
    let input = quote! {
        struct Input(u32, Vec<u8>, ());
    };

    let Ok(DeriveInput { ident, data, generics, .. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl ::des::net::message::MessageBody for Input {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.0) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.1) +
                    <() as ::des::net::message::MessageBody>::byte_len(&self.2) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_unnamed_generic_nonbounded() {
    let input = quote! {
        struct Input<T>(u32, Vec<u8>, T);
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.0) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.1) +
                    <T as ::des::net::message::MessageBody>::byte_len(&self.2) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_unnamed_generic_bounded() {
    let input = quote! {
        struct Input<T: Copy + Eq>(u32, Vec<u8>, T);
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: Copy + Eq + ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.0) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.1) +
                    <T as ::des::net::message::MessageBody>::byte_len(&self.2) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn struct_unnamed_generic_where_clause() {
    let input = quote! {
        struct Input<T>(u32, Vec<u8>, T) where T: Copy + std::hash::Hash;
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T>
            where T: Copy + std::hash::Hash {
                fn byte_len(&self) -> usize {
                    <u32 as ::des::net::message::MessageBody>::byte_len(&self.0) +
                    <Vec<u8> as ::des::net::message::MessageBody>::byte_len(&self.1) +
                    <T as ::des::net::message::MessageBody>::byte_len(&self.2) +
                    0
                }
            }
        }
        .to_string()
    );
}

#[test]
fn enum_unit_fields() {
    let input = quote! {
        enum Input {
            A,
            B,
            CVariant,
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl ::des::net::message::MessageBody for Input {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A => 0,
                          Input::B => 0,
                          Input::CVariant => 0,
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn enum_unnamed_fields() {
    let input = quote! {
        enum Input {
            A(u32),
            B(Vec<u8>),
            CVariant(f64, f32),
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl ::des::net::message::MessageBody for Input {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A(v0,) => <u32 as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::B(v0,) => <Vec<u8> as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::CVariant(v0, v1, ) => 
                            <f64 as ::des::net::message::MessageBody>::byte_len(v0) +
                            <f32 as ::des::net::message::MessageBody>::byte_len(v1) + 
                            0,
                    }
                }
            }
        }
        .to_string()
    );
}


#[test]
fn enum_unnamed_fields_generic_unbounded() {
    let input = quote! {
        enum Input<T> {
            A(T),
            B(Vec<u8>),
            CVariant(f64, T),
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A(v0,) => <T as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::B(v0,) => <Vec<u8> as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::CVariant(v0, v1, ) => 
                            <f64 as ::des::net::message::MessageBody>::byte_len(v0) +
                            <T as ::des::net::message::MessageBody>::byte_len(v1) + 
                            0,
                    }
                }
            }
        }
        .to_string()
    );
}


#[test]
fn enum_unnamed_fields_generic_bounded() {
    let input = quote! {
        enum Input<T: Copy> {
            A(T),
            B(Vec<u8>),
            CVariant(f64, T),
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: Copy + ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A(v0,) => <T as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::B(v0,) => <Vec<u8> as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::CVariant(v0, v1, ) => 
                            <f64 as ::des::net::message::MessageBody>::byte_len(v0) +
                            <T as ::des::net::message::MessageBody>::byte_len(v1) + 
                            0,
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn enum_unnamed_fields_generic_where_clause() {
    let input = quote! {
        enum Input<T> where T: std::hash::Hash {
            A(T),
            B(Vec<u8>),
            CVariant(f64, T),
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> 
             where T: std::hash::Hash {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A(v0,) => <T as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::B(v0,) => <Vec<u8> as ::des::net::message::MessageBody>::byte_len(v0) + 0,
                          Input::CVariant(v0, v1, ) => 
                            <f64 as ::des::net::message::MessageBody>::byte_len(v0) +
                            <T as ::des::net::message::MessageBody>::byte_len(v1) + 
                            0,
                    }
                }
            }
        }
        .to_string()
    );
}

// 

#[test]
fn enum_named_fields() {
    let input = quote! {
        enum Input {
            A { x: u32},
            B { y: Vec<u8>, z: f64},
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl ::des::net::message::MessageBody for Input {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A { ref x, } => <u32 as ::des::net::message::MessageBody>::byte_len(x) + 0,
                          Input::B { ref y, ref z, } => 
                          <Vec<u8> as ::des::net::message::MessageBody>::byte_len(y) +
                          <f64 as ::des::net::message::MessageBody>::byte_len(z) + 
                          0,
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn enum_named_fields_generic_unbounded() {
    let input = quote! {
        enum Input<T> {
            A { x: T },
            B { y: Vec<T>, z: f64},
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A { ref x, } => <T as ::des::net::message::MessageBody>::byte_len(x) + 0,
                          Input::B { ref y, ref z, } => 
                          <Vec<T> as ::des::net::message::MessageBody>::byte_len(y) +
                          <f64 as ::des::net::message::MessageBody>::byte_len(z) + 
                          0,
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn enum_named_fields_generic_bounded() {
    let input = quote! {
        enum Input<T: Copy> {
            A { x: T },
            B { y: Vec<T>, z: f64},
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: Copy + ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A { ref x, } => <T as ::des::net::message::MessageBody>::byte_len(x) + 0,
                          Input::B { ref y, ref z, } => 
                          <Vec<T> as ::des::net::message::MessageBody>::byte_len(y) +
                          <f64 as ::des::net::message::MessageBody>::byte_len(z) + 
                          0,
                    }
                }
            }
        }
        .to_string()
    );
}

#[test]
fn enum_named_fields_generic_where_clause() {
    let input = quote! {
        enum Input<T> where T: Copy {
            A { x: T },
            B { y: Vec<T>, z: f64},
        }
    };

    let Ok(DeriveInput { ident, data, generics ,.. }) = parse2(input) else {
        panic!("Failed to parse input steam")
    };
    let Ok(output) = des_macros_core::message_body_derive_impl(ident, data, generics) else {
        panic!("Failed with diagnostic")
    };

    assert_eq!(
        output.to_string(),
        quote! {
            impl<T: ::des::net::message::MessageBody> ::des::net::message::MessageBody for Input<T> where T: Copy {
                fn byte_len(&self) -> usize {
                    match self {
                          Input::A { ref x, } => <T as ::des::net::message::MessageBody>::byte_len(x) + 0,
                          Input::B { ref y, ref z, } => 
                          <Vec<T> as ::des::net::message::MessageBody>::byte_len(y) +
                          <f64 as ::des::net::message::MessageBody>::byte_len(z) + 
                          0,
                    }
                }
            }
        }
        .to_string()
    );
}