//! Optional instruction-set extensions the generated code may use. What the generator relied
//! on is recorded in the image header, and an image is only loaded where all of it is present.

/// Bit set of [`POPCNT`], ….
pub type Features = u32;

pub const POPCNT: Features = 1;

/// Extensions of the machine running this process.
pub fn host() -> Features {
    let mut f = 0;
    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("popcnt") {
        f |= POPCNT;
    }
    f
}

/// Whether code generated now may use `popcnt`.
pub fn popcnt() -> bool {
    host() & POPCNT != 0
}
