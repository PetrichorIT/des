#[cfg(feature = "simtime_u128")]
pub use self::u128::*;
#[cfg(feature = "simtime_u128")]
mod u128;

#[cfg(not(feature = "simtime_u128"))]
pub use self::f64::*;
#[cfg(not(feature = "simtime_u128"))]
mod f64;
