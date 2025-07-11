use std::str::FromStr;

use anyhow::{bail, Error};
use bitcoin::Amount;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct FeeRate(pub f64);

impl FromStr for FeeRate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(f64::from_str(s)?)
    }
}

impl TryFrom<&[u8]> for FeeRate {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(f64::from_le_bytes(bytes[..8].try_into()?)))
    }
}

impl TryFrom<f64> for FeeRate {
    type Error = Error;

    fn try_from(rate: f64) -> Result<Self, Self::Error> {
        if rate.is_sign_negative() | rate.is_nan() | rate.is_infinite() {
            bail!("invalid fee rate: {rate}")
        }
        Ok(Self(rate))
    }
}

impl FeeRate {
    pub fn fee(&self, vsize: usize) -> Amount {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        Amount::from_sat((self.0 * vsize as f64).round() as u64)
    }

    pub fn n(&self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!("1.1".parse::<FeeRate>().unwrap().0, 1.1);
        assert_eq!("11.19".parse::<FeeRate>().unwrap().0, 11.19);
        assert_eq!("11.1111".parse::<FeeRate>().unwrap().0, 11.1111);
        assert!("-4.2".parse::<FeeRate>().is_err());
        assert!(FeeRate::try_from(f64::INFINITY).is_err());
        assert!(FeeRate::try_from(f64::NAN).is_err());
    }

    #[test]
    fn fee() {
        assert_eq!(
            "2.5".parse::<FeeRate>().unwrap().fee(100),
            Amount::from_sat(250)
        );
        assert_eq!(
            "2.0".parse::<FeeRate>().unwrap().fee(1024),
            Amount::from_sat(2048)
        );
        assert_eq!(
            "1.1".parse::<FeeRate>().unwrap().fee(100),
            Amount::from_sat(110)
        );
        assert_eq!(
            "1.0".parse::<FeeRate>().unwrap().fee(123456789),
            Amount::from_sat(123456789)
        );
    }
}
