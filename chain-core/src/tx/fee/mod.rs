//! # Fee calculation and fee algorithms
//! adapted from https://github.com/input-output-hk/rust-cardano (Cardano Rust)
//! Copyright (c) 2018, Input Output HK (licensed under the MIT License)
//! Modifications Copyright (c) 2018 - 2019, Foris Limited (licensed under the Apache License, Version 2.0)

use crate::init::coin::{Coin, CoinError};
use crate::tx::TxAux;
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::num::ParseIntError;
use std::ops::{Add, Div, Mul};
use std::prelude::v1::Vec;
use std::str::FromStr;
use std::{error, fmt};

/// A fee value that represent either a fee to pay, or a fee paid.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Encode, Decode)]
pub struct Fee(Coin);

impl Fee {
    pub fn new(coin: Coin) -> Self {
        Fee(coin)
    }

    pub fn to_coin(self) -> Coin {
        self.0
    }
}

/// Represents a 3 digit fixed decimal
/// TODO: overflow checks in Cargo?
/// [profile.release]
/// overflow-checks = true
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "serde", serde(transparent))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Milli(u64);

impl Milli {
    /// takes the integer part and 3-digit fractional part
    /// and returns the 3-digit fixed decimal number (i.fff)
    #[inline]
    pub const fn new(i: u64, f: u64) -> Self {
        Milli(i * 1000 + f % 1000)
    }

    /// takes the integer part
    /// and returns the 3-digit fixed decimal number (i.000)
    #[inline]
    pub fn integral(i: u64) -> Self {
        Milli(i * 1000)
    }

    pub fn to_integral(self) -> u64 {
        // note that we want the ceiling
        if self.0 % 1000 == 0 {
            self.0 / 1000
        } else {
            (self.0 / 1000) + 1
        }
    }

    #[inline]
    pub fn to_integral_trunc(self) -> u64 {
        self.0 / 1000
    }

    #[inline]
    pub fn as_millis(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn from_millis(millis: u64) -> Milli {
        Milli(millis)
    }

    pub fn sqrt(self) -> Milli {
        let mut sqrt = Milli(1000); // Initial estimate of sqrt = 1.0
        let mut improved_sqrt = self.improve_sqrt(sqrt);

        let mut difference = sqrt.diff(improved_sqrt);

        while difference > Milli(1) {
            sqrt = improved_sqrt;
            improved_sqrt = self.improve_sqrt(sqrt);

            difference = sqrt.diff(improved_sqrt);
        }

        improved_sqrt
    }

    fn improve_sqrt(self, estimate: Milli) -> Milli {
        let result = estimate + (self / estimate);
        Milli(result.0 >> 1)
    }

    fn diff(self, rhs: Milli) -> Milli {
        if self > rhs {
            Milli(self.0 - rhs.0)
        } else {
            Milli(rhs.0 - self.0)
        }
    }
}

#[derive(Debug)]
pub enum MilliError {
    /// An invalid length of parts (should be either 1 or 2)
    InvalidPartsLength(usize),
    /// Number parsing error
    InvalidInteger(ParseIntError),
}

impl fmt::Display for MilliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            MilliError::InvalidPartsLength(len) => {
                write!(f, "Invalid parts length: {} (2 expected)", len)
            }
            MilliError::InvalidInteger(ref err) => write!(f, "Integer parsing error: {}", err),
        }
    }
}

impl From<ParseIntError> for MilliError {
    fn from(err: ParseIntError) -> Self {
        MilliError::InvalidInteger(err)
    }
}

impl FromStr for Milli {
    type Err = MilliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('.').collect::<Vec<&str>>();
        let len = parts.len();

        let (integral, fractional) = match len {
            1 => (parts[0].parse()?, 0),
            2 => (parts[0].parse()?, format!("{:0<3}", parts[1]).parse()?),
            _ => return Err(MilliError::InvalidPartsLength(len)),
        };

        Ok(Milli::new(integral, fractional))
    }
}

impl fmt::Display for Milli {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let integral = self.0 / 1000;
        let fractional = self.0 % 1000;
        write!(f, "{}.{:0>3}", integral, fractional)
    }
}

impl error::Error for MilliError {
    fn description(&self) -> &str {
        "Milli parsing error"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            MilliError::InvalidInteger(ref err) => Some(err),
            _ => None,
        }
    }
}

impl Add for Milli {
    type Output = Milli;
    fn add(self, other: Self) -> Self {
        Milli(self.0 + other.0)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for Milli {
    type Output = Milli;

    fn mul(self, other: Self) -> Self {
        let v = u128::from(self.0) * u128::from(other.0);
        Milli((v / 1000) as u64)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Milli {
    type Output = Milli;

    fn div(self, rhs: Milli) -> Self::Output {
        let numerator: u128 = u128::from(self.0) * 1000;
        let denominator = u128::from(rhs.0);

        let result = numerator / denominator;

        Milli(result as u64)
    }
}

/// Linear fee using the basic affine formula `COEFFICIENT * scale_bytes(txaux).len() + CONSTANT`
#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LinearFee {
    /// this is the minimal fee
    pub constant: Milli,
    /// the transaction's size coefficient fee
    pub coefficient: Milli,
}

impl LinearFee {
    pub fn new(constant: Milli, coefficient: Milli) -> Self {
        LinearFee {
            constant,
            coefficient,
        }
    }

    pub fn estimate(&self, sz: usize) -> Result<Fee, CoinError> {
        let msz = Milli::integral(sz as u64);
        let fee = self.constant + self.coefficient * msz;
        let coin = Coin::new(fee.to_integral())?;
        Ok(Fee(coin))
    }
}

/// Calculation of fees for a specific chosen algorithm
pub trait FeeAlgorithm: Send + Sync {
    fn calculate_fee(&self, num_bytes: usize) -> Result<Fee, CoinError>;
    fn calculate_for_txaux(&self, txaux: &TxAux) -> Result<Fee, CoinError>;
}

impl FeeAlgorithm for LinearFee {
    fn calculate_fee(&self, num_bytes: usize) -> Result<Fee, CoinError> {
        self.estimate(num_bytes)
    }

    fn calculate_for_txaux(&self, txaux: &TxAux) -> Result<Fee, CoinError> {
        self.estimate(txaux.encode().len())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::quickcheck;

    fn test_milli_add_eq(v1: u64, v2: u64) -> bool {
        let v = v1 + v2;
        let n1 = Milli::new(v1 / 1000, v1 % 1000);
        let n2 = Milli::new(v2 / 1000, v2 % 1000);
        let n = n1 + n2;
        assert_eq!(v / 1000, n.to_integral_trunc());
        v / 1000 == n.to_integral_trunc()
    }

    fn test_milli_mul_eq(v1: u64, v2: u64) -> bool {
        let v = v1 as u128 * v2 as u128;
        let n1 = Milli::new(v1 / 1000, v1 % 1000);
        let n2 = Milli::new(v2 / 1000, v2 % 1000);
        let n = n1 * n2;
        assert_eq!((v / 1000000) as u64, n.to_integral_trunc());
        (v / 1000000) as u64 == n.to_integral_trunc()
    }

    fn test_milli_div_eq(v1: u64, v2: u64) -> bool {
        let v = (v1 as u128 * 1000) / v2 as u128;

        let n1 = Milli::new(v1 / 1000, v1 % 1000);
        let n2 = Milli::new(v2 / 1000, v2 % 1000);
        let n = n1 / n2;

        assert_eq!(v as u64, n.as_millis());
        v as u64 == n.as_millis()
    }

    #[test]
    fn check_fee_add() {
        test_milli_add_eq(10124128_192, 802_504);
        test_milli_add_eq(1124128_915, 124802_192);
        test_milli_add_eq(241, 900001_901);
        test_milli_add_eq(241, 407);
    }

    #[test]
    fn check_fee_mul() {
        test_milli_mul_eq(10124128_192, 802_192);
        test_milli_mul_eq(1124128_192, 124802_192);
        test_milli_mul_eq(241, 900001_900);
        test_milli_mul_eq(241, 400);
    }

    #[test]
    fn check_fee_div() {
        test_milli_div_eq(10124128_192, 802_192);
        test_milli_div_eq(1124128_192, 124802_192);
        test_milli_div_eq(241, 900001_900);
        test_milli_div_eq(241, 400);
    }

    #[test]
    fn check_milli_from_str() {
        assert_eq!(1000, Milli::from_str("1").unwrap().as_millis());
        assert_eq!(1000, Milli::from_str("1.0").unwrap().as_millis());
        assert_eq!(1100, Milli::from_str("1.1").unwrap().as_millis());
        assert_eq!(1150, Milli::from_str("1.15").unwrap().as_millis());
    }

    #[test]
    fn check_milli_sqrt() {
        assert_eq!(Milli::new(0, 100), Milli::new(0, 10).sqrt());
        assert_eq!(Milli::new(1, 414), Milli::new(2, 0).sqrt());
        assert_eq!(Milli::new(1, 732), Milli::new(3, 0).sqrt());
        assert_eq!(Milli::new(2, 0), Milli::new(4, 0).sqrt());
    }

    fn approx_eq(v1: Milli, v2: Milli, delta: Milli) -> bool {
        v1.diff(v2) <= delta
    }

    quickcheck! {
        fn prop_milli_add(n1: u64, n2: u64) -> bool {
            test_milli_add_eq(n1, n2)
        }

        fn prop_milli_mul(n1: u64, n2: u64) -> bool {
            test_milli_mul_eq(n1, n2)
        }

        fn prop_milli_div(n1: u64, n2: u64) -> bool {
            n2 == 0 || test_milli_div_eq(n1, n2)
        }

        fn prop_symm(n1: u64, n2: u64) -> bool {
            // Check Symmetry:  a*b = b*a
            let v1 = Milli::from_millis(n1);
            let v2 = Milli::from_millis(n2);
            v1 * v2 == v2 * v1
        }

        fn prop_milli_tri_ineq(n1: u64, n2: u64, n3: u64) -> bool {
            //Check: Triangle-inequality
            let v1 = Milli::from_millis(n1);
            let v2 = Milli::from_millis(n2);
            let v3 = Milli::from_millis(n3);
            let v11 = v1 * v1;
            let v22 = v2 * v2;
            let v33 = v3 * v3;
            if v1 + v2 + v3 >= Milli::new(1, 0) {
                v11 + v22 + v33 >= (v11 + v22 + v33).sqrt()
            } else {
                v11 + v22 + v33 <= (v11 + v22 + v33).sqrt()
            }
        }

        fn prop_milli_mul_inverse(n1: u64, n2: u64) -> bool {
            // Check: (a/b)*b = a
            let v1 = Milli::from_millis(n1);
            let v2 = Milli::from_millis(n2);
            v2 == Milli::new(0, 0) || approx_eq((v1 / v2) * v2, v1, Milli::from_millis(1))
        }

        fn propmilli_sq_mul(n1: u64) -> bool {
            //Check: sqrt(a*a) = a
            let v1 = Milli::from_millis(n1);
            approx_eq(v1.sqrt() * v1.sqrt(), v1, Milli::from_millis(1))
        }

        fn prop_milli_mul_id(n1: u64) -> bool {
            //Check: a * 1 = a
            let v1 = Milli::from_millis(n1);
            v1 * Milli::new(1, 0) == v1
        }

        fn prop_milli_div_id(n1: u64) -> bool {
            //Check: a/a = 1
            if n1 == 0 {
                true
            } else {
                let v1 = Milli::from_millis(n1);
                v1 / v1 == Milli::new(1, 0)
            }
        }
    }
}
