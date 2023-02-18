/// Creates a NDL registry from the given types.
#[macro_export]
macro_rules! registry {
    ($($t:ty),*) => {{
        let mut registry = $crate::ndl::Registry::new();
        $(
            registry.add(stringify!($t), Box::new(|| Box::new(<$t as Module>::new())));
        )*
        registry
    }};
}
