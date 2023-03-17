use super::Par;
use crate::net::module::module_path;
use std::marker::PhantomData;

///
/// Returns a parameter by reference (not parsed).
///
#[must_use]
pub fn par(key: &str) -> Par {
    Par {
        key: format!("{}.{key}", module_path()),
        value: None,
        _phantom: PhantomData,
    }
}

///
/// Returns a parameter by reference (not parsed).
///
pub fn par_for(key: &str, module: &str) -> Par {
    Par {
        key: format!("{module}.{key}"),
        value: None,
        _phantom: PhantomData,
    }
}

// ///
// /// Returns the parameters for the current module.
// ///
// #[must_use]
// pub fn pars() -> HashMap<String, String> {
//     // let path = self::module_path();
//     // globals().parameters.get_def_table(path.as_str())
//     todo!()
// }
