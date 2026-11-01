//! Defined integer semantics for source-derived operations; no floating-point physics dependency.
use crate::{Error, Result};
pub fn wrapping_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}
pub fn wrapping_mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}
pub fn q16_mul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 16) as i32
}
pub fn floor_q16(value: i32) -> i32 {
    value >> 16
}
pub fn divide(a: i32, b: i32) -> Result<i32> {
    a.checked_div(b)
        .ok_or(Error::Invalid("division by zero/overflow"))
}
pub fn truncate_shift(a: i32, b: u32) -> Result<i32> {
    if b > 31 {
        return Err(Error::Invalid("shift range"));
    }
    Ok((i64::from(a) / (1i64 << b)) as i32)
}
