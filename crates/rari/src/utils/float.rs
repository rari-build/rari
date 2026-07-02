//! Integer-to-float conversions where reduced precision is acceptable.

#[expect(clippy::cast_precision_loss)]
#[inline]
#[must_use]
pub const fn u32_to_f32(value: u32) -> f32 {
    value as f32
}

#[expect(clippy::cast_precision_loss)]
#[inline]
#[must_use]
pub const fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

#[expect(clippy::cast_precision_loss)]
#[inline]
#[must_use]
pub const fn usize_to_f32(value: usize) -> f32 {
    value as f32
}

#[expect(clippy::cast_precision_loss)]
#[inline]
#[must_use]
pub fn u64_ratio(numerator: u64, denominator: u64) -> f64 {
    numerator as f64 / denominator as f64
}
