/// Creates a registry of types that implement [`Module`](crate::net::module::Module),
/// to link rust structs to NDL modules.
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
///
/// # use des_ndl::error::RootResult as Result;
/// fn main() -> Result<()> {
///     let registry = registry![DnsServer, Client, Server];
///     # return Ok(());
///     let app = Sim::ndl("path/to/ndl", registry)?;
///     let rt = Builder::new().build(app);
///     let r = rt.run();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    ($($t:ty),*) => {{
        use $crate::ndl::RegistryCreatable;
        use $crate::net::module::Module;

        let mut registry = $crate::ndl::Registry::new();
        $(
            registry = registry.symbol(stringify!($t), |path| <$t as RegistryCreatable>::create(path, stringify!($t)));
        )*

        registry
    }};
}
