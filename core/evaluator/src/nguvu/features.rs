//! Optional instruction-set extensions the generated code may use. What the generator relied
//! on is recorded in the image header, and an image is only loaded where all of it is present.

/// Bit set of [`POPCNT`], ….
pub type Features = u32;

/// A population-count instruction (`popcnt` on x86-64, `cnt` on AArch64).
pub const POPCNT: Features = 1;

/// Extensions of the machine running this process.
pub fn host() -> Features {
    let mut f = 0;
    #[cfg(target_arch = "x86_64")]
    if std::arch::is_x86_feature_detected!("popcnt") {
        f |= POPCNT;
    }
    // AArch64 counts bits with Advanced SIMD (`cnt`), which every AArch64 Linux/macOS CPU has.
    #[cfg(target_arch = "aarch64")]
    {
        f |= POPCNT;
    }
    f
}

/// Whether code generated now may use `popcnt`.
pub fn popcnt() -> bool {
    host() & POPCNT != 0
}
