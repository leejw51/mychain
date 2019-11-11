use std::fmt;

use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::de;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::tx::data::TxId;

// TODO: u16 and Vec size check in Decode implementation
pub type TxoIndex = u16;

/// Structure used for addressing a specific output of a transaction
/// built from a TxId (hash of the tx) and the offset in the outputs of this
/// transaction.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Encode, Decode)]
#[cfg_attr(
    all(feature = "serde", feature = "hex"),
    derive(Serialize, Deserialize)
)]
pub struct TxoPointer {
    #[cfg_attr(
        all(feature = "serde", feature = "hex"),
        serde(serialize_with = "serialize_transaction_id")
    )]
    #[cfg_attr(
        all(feature = "serde", feature = "hex"),
        serde(deserialize_with = "deserialize_transaction_id")
    )]
    pub id: TxId,
    // TODO: u16 and Vec size check in Decode implementation
    pub index: TxoIndex,
}

#[cfg(all(feature = "serde", feature = "hex"))]
fn serialize_transaction_id<S>(
    transaction_id: &TxId,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&hex::encode(transaction_id))
}

#[cfg(all(feature = "serde", feature = "hex"))]
fn deserialize_transaction_id<'de, D>(deserializer: D) -> std::result::Result<TxId, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor;

    impl<'de> de::Visitor<'de> for StrVisitor {
        type Value = TxId;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("transaction id in hexadecimal string")
        }

        #[inline]
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let transaction_id_vec =
                hex::decode(value).map_err(|err| de::Error::custom(err.to_string()))?;
            if transaction_id_vec.len() != 32 {
                return Err(de::Error::custom(format!(
                    "Invalid transaction id length: {}",
                    transaction_id_vec.len()
                )));
            }

            let mut transaction_id = [0; 32];
            transaction_id.copy_from_slice(&transaction_id_vec);

            Ok(transaction_id)
        }
    }

    deserializer.deserialize_str(StrVisitor)
}

impl fmt::Display for TxoPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{}", self.id, self.index)
    }
}

impl TxoPointer {
    /// Constructs a new TX input (mainly for testing/tools).
    pub fn new(id: TxId, index: usize) -> Self {
        TxoPointer {
            id,
            index: index as TxoIndex,
        }
    }
}
