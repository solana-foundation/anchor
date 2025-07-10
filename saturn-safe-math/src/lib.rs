use num::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub};

use primitive_types::U256;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MathError {
    #[error("Addition overflowed")]
    AdditionOverflow,
    #[error("Subtraction overflowed")]
    SubtractionOverflow,
    #[error("Multiplication overflowed")]
    MultiplicationOverflow,
    #[error("Division overflowed")]
    DivisionOverflow,
    #[error("Failed to convert a math result from U256 back to its original type")]
    ConversionError,
}

pub fn safe_add<T>(a: T, b: T) -> Result<T, MathError>
where
    T: CheckedAdd,
{
    a.checked_add(&b).ok_or(MathError::AdditionOverflow)
}

pub fn safe_sub<T>(a: T, b: T) -> Result<T, MathError>
where
    T: CheckedSub,
{
    a.checked_sub(&b).ok_or(MathError::SubtractionOverflow)
}

pub fn safe_mul<T>(a: T, b: T) -> Result<T, MathError>
where
    T: CheckedMul,
{
    a.checked_mul(&b).ok_or(MathError::MultiplicationOverflow)
}

pub fn safe_div<T>(a: T, b: T) -> Result<T, MathError>
where
    T: CheckedDiv,
{
    a.checked_div(&b).ok_or(MathError::DivisionOverflow)
}

pub fn mul_div<T>(mul_a: T, mul_b: T, div: T) -> Result<T, MathError>
where
    T: TryFrom<U256>,
    U256: From<T>,
{
    let a = U256::from(mul_a);
    let b = U256::from(mul_b);

    let mul = a.checked_mul(b).ok_or(MathError::MultiplicationOverflow)?;
    let div = U256::from(div);

    let res = mul.checked_div(div).ok_or(MathError::DivisionOverflow)?;

    res.try_into().map_err(|_| MathError::ConversionError)
}
