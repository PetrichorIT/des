/// Creates a registry of types that implement [`Module`](crate::net::module::Module),
/// to link rust structs to NDL modules.
///
/// The listing of types can be optionally suffixed with
/// `else <some_type>`  to declare a fallback module
/// in the [`Registry`](crate::ndl::Registry). The suffix `else _`
/// declarse the default fallback module.
///
/// # Example
///
/// ```rust
/// # use des::prelude::*;
/// # use des::registry;
/// #[derive(Default)]
/// struct DnsServer;
/// /* ... */
/// # impl Module for DnsServer {}
/// #[derive(Default)]
/// struct Client;
/// /* ... */
/// # impl Module for Client {}
/// #[derive(Default)]
/// struct Server;
/// # impl Module for Server {}
/// /* ... */
/// # use des_net_utils::ndl::error::Result;
/// fn main() -> Result<()> {
///     let registry = registry![DnsServer, Client, Server, else _];
///     # return Ok(());
///     let app = Sim::ndl("path/to/ndl", registry)?;
///     let rt = Builder::new().build(app);
///     let r = rt.run();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    ($($t:ty),*) => {{
        let registry = $crate::net::ndl::Registry::new();
        $(
            let registry = registry.symbol::<$t>(stringify!($t));
        )*

        registry
    }};

    ($($t:ty),*, else _) => {{
        let registry = $crate::net::ndl::Registry::new();
        $(
            let registry = registry.symbol::<$t>(stringify!($t));
        )*

        registry.with_default_fallback()
    }};

    ($($t:ty),*, else $f:ty) => {{
        let registry = $crate::net::ndl::Registry::new();
        $(
            let registry = registry.symbol::<$t>(stringify!($t));
        )*

        registry.with_fallback(|| <$f as std::default::Default>::default())
    }};
}
