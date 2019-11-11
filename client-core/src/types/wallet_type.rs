//! Type for specifying different wallet types
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use unicase::eq_ascii;

use client_common::{Error, ErrorKind, Result};

/// Enum for specifying the kind of wallet (e.g., `Basic`, `HD`)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum WalletKind {
    /// Basic Wallet
    Basic,
    /// HD Wallet
    HD,
}

impl FromStr for WalletKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if eq_ascii(s, "hd") {
            Ok(WalletKind::HD)
        } else if eq_ascii(s, "basic") {
            Ok(WalletKind::Basic)
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "Wallet type can either be `hd` or `basic`",
            ))
        }
    }
}

impl Default for WalletKind {
    fn default() -> Self {
        WalletKind::Basic
    }
}
