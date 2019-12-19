#[cfg(not(feature = "mesalock_sgx"))]
use std::fmt;
#[cfg(not(feature = "mesalock_sgx"))]
use std::str::FromStr;

use parity_scale_codec::{Decode, Encode};
#[cfg(not(feature = "mesalock_sgx"))]
use serde::de;
#[cfg(not(feature = "mesalock_sgx"))]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::common::Timespec;
use crate::init::coin::Coin;
use crate::tx::data::address::ExtendedAddr;

/// Tx Output composed of an address and a coin value
/// TODO: custom Encode/Decode when data structures are finalized (for backwards/forwards compatibility, encoders/decoders should be able to work with old formats)
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(not(feature = "mesalock_sgx"), derive(Serialize, Deserialize))]
pub struct TxOut {
    #[cfg_attr(
        not(feature = "mesalock_sgx"),
        serde(serialize_with = "serialize_address")
    )]
    #[cfg_attr(
        not(feature = "mesalock_sgx"),
        serde(deserialize_with = "deserialize_address")
    )]
    pub address: ExtendedAddr,
    pub value: Coin,
    pub valid_from: Option<Timespec>,
}

#[cfg(not(feature = "mesalock_sgx"))]
fn serialize_address<S>(
    address: &ExtendedAddr,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&address.to_string())
}

#[cfg(not(feature = "mesalock_sgx"))]
fn deserialize_address<'de, D>(deserializer: D) -> std::result::Result<ExtendedAddr, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor;

    impl<'de> de::Visitor<'de> for StrVisitor {
        type Value = ExtendedAddr;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("transfer address in bech32 format")
        }

        #[inline]
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            ExtendedAddr::from_str(value).map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    deserializer.deserialize_str(StrVisitor)
}

#[cfg(not(feature = "mesalock_sgx"))]
impl fmt::Display for TxOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.address, self.value)
    }
}

impl TxOut {
    /// creates a TX output (mainly for testing/tools)
    pub fn new(address: ExtendedAddr, value: Coin) -> Self {
        TxOut {
            address,
            value,
            valid_from: None,
        }
    }

    /// creates a TX output with timelock
    pub fn new_with_timelock(address: ExtendedAddr, value: Coin, valid_from: Timespec) -> Self {
        TxOut {
            address,
            value,
            valid_from: Some(valid_from),
        }
    }
}
