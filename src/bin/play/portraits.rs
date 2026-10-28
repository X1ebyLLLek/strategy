//! The portraits built into the game (`assets/portraits`, by build.rs): `g000`… generals, `c000`…
//! people in civil dress.

include!(concat!(env!("OUT_DIR"), "/portraits.rs"));

/// How many portraits there are whose names begin with `prefix` (`g`: generals, `c`: people in
/// civil dress).
pub fn count(prefix: &str) -> usize {
    PORTRAITS.iter().filter(|f| f.0.starts_with(prefix)).count()
}

/// A portrait's picture (JPEG) by its name (the window draws it).
#[allow(dead_code)]
pub fn picture(name: &str) -> Option<&'static [u8]> {
    PORTRAITS.iter().find(|f| f.0 == name).map(|f| f.1)
}
