//! Conversions where truncation is acceptable or explicitly handled.

use std::time::Duration;

#[inline]
#[must_use]
pub fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[inline]
#[must_use]
pub fn u64_to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[inline]
#[must_use]
pub fn u32_to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

#[cfg(test)]
#[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
#[inline]
#[must_use]
pub fn usize_fraction_ceil(value: usize, factor: f64) -> usize {
    (value as f64 * factor).ceil() as usize
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[inline]
#[must_use]
pub fn f64_floor_usize(value: f64) -> usize {
    value.floor() as usize
}

#[expect(clippy::cast_possible_truncation)]
#[inline]
#[must_use]
pub const fn u16_to_u8(value: u16) -> u8 {
    value as u8
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[inline]
#[must_use]
pub const fn f32_to_u32(value: f32) -> u32 {
    value as u32
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[inline]
#[must_use]
pub const fn f32_to_u8(value: f32) -> u8 {
    value as u8
}

#[expect(clippy::cast_possible_truncation)]
#[inline]
#[must_use]
pub const fn f32_to_i32(value: f32) -> i32 {
    value as i32
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[inline]
#[must_use]
pub const fn f32_to_usize(value: f32) -> usize {
    value as usize
}

#[expect(clippy::cast_possible_truncation)]
#[inline]
#[must_use]
pub const fn f64_to_f32(value: f64) -> f32 {
    value as f32
}
