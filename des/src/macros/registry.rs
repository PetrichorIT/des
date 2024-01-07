/// Creates a registry of types that implement [`Module`](crate::net::module::Module),
/// to link rust structs to NDL modules.
///
/// # Example
///
/// ```rust
/// # use des::prelude::*;
/// # use des::registry;
/// struct DnsServer;
/// /* ... */
/// # impl Module for DnsServer { fn new() -> Self { Self }}
///
/// struct Client;
/// /* ... */
/// # impl Module for Client { fn new() -> Self { Self }}
///
/// struct Server;
/// # impl Module for Server { fn new() -> Self { Self }}
/// /* ... */
///
/// # use des_ndl::error::RootResult as Result;
/// fn main() -> Result<()> {
///     let registry = registry![DnsServer, Client, Server];
///     # return Ok(());
///     let app = NdlApplication::new("path/to/ndl", registry)?;
///     let rt = Builder::new().build(NetworkApplication::new(app));
///     let r = rt.run();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    ($($t:ty),*) => {{
        let mut registry = $crate::ndl::Registry::new();
        $(
            registry.add(stringify!($t), Box::new(|| <$t as Module>::new().as_processing_chain()));
        )*
        registry
    }};
}
