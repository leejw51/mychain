use parity_scale_codec::{Decode, Encode};
use secstr::SecUtf8;

use chain_core::init::network::get_network;
use client_common::storage::decrypt_bytes;
use client_common::{
    Error, ErrorKind, PrivateKey, PublicKey, Result, ResultExt, SecureStorage, Storage,
};

use crate::types::AddressType;
use crate::{HDSeed, Mnemonic};

const KEYSPACE: &str = "core_hd_key";

#[derive(Debug, PartialEq, Encode, Decode)]
struct HdKey {
    staking_index: u32,
    transfer_index: u32,
    seed: HDSeed,
}

/// Stores HD Wallet's `seed` and `index`
#[derive(Debug, Default, Clone)]
pub struct HdKeyService<T: Storage> {
    storage: T,
}

impl<T> HdKeyService<T>
where
    T: Storage,
{
    /// Creates a new instance of HD key service
    #[inline]
    pub fn new(storage: T) -> Self {
        Self { storage }
    }

    /// Returns true if wallet's HD key is present in storage
    pub fn has_wallet(&self, name: &str) -> Result<bool> {
        self.storage.contains_key(KEYSPACE, name)
    }

    /// Adds a new mnemonic in storage and sets its index to zero
    pub fn add_mnemonic(
        &self,
        name: &str,
        mnemonic: &Mnemonic,
        passphrase: &SecUtf8,
    ) -> Result<()> {
        if self.storage.get(KEYSPACE, name)?.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "HD Key with given name already exists",
            ));
        }

        let hd_seed = HDSeed::from(mnemonic);

        let hd_key = HdKey {
            staking_index: 0,
            transfer_index: 0,
            seed: hd_seed,
        };

        self.storage
            .set_secure(KEYSPACE, name, hd_key.encode(), passphrase)
            .map(|_| ())
    }

    /// Generates keypair for given wallet and address type
    ///
    /// # Note
    ///
    /// Key chain path format: `m / purpose' / coin_type' / account' / change / address_index`
    ///
    /// - `purpose`: `44`
    /// - `coin_type`: `394` for mainnet and `1` for others
    /// - `account`: `0` for `AddressType::Transfer` and `1` for `AddressType::Staking`
    /// - `change`: `0`
    /// - `address_index`: Index of address as retrieved from storage
    pub fn generate_keypair(
        &self,
        name: &str,
        passphrase: &SecUtf8,
        address_type: AddressType,
    ) -> Result<(PublicKey, PrivateKey)> {
        let bytes = self
            .storage
            .fetch_and_update_secure(KEYSPACE, name, passphrase, |bytes| {
                let mut hd_key_bytes = bytes.chain(|| {
                    (
                        ErrorKind::InvalidInput,
                        format!("HD Key with name ({}) not found", name),
                    )
                })?;

                let mut hd_key = HdKey::decode(&mut hd_key_bytes).chain(|| {
                    (
                        ErrorKind::DeserializationError,
                        "Unable to deserialize HD Key from bytes",
                    )
                })?;

                match address_type {
                    AddressType::Staking => hd_key.staking_index += 1,
                    AddressType::Transfer => hd_key.transfer_index += 1,
                }

                Ok(Some(hd_key.encode()))
            })?
            .chain(|| {
                (
                    ErrorKind::InvalidInput,
                    format!("HD Key with name ({}) not found", name),
                )
            })?;

        let hd_key_bytes = decrypt_bytes(name, passphrase, &bytes)?;
        let hd_key = HdKey::decode(&mut hd_key_bytes.as_slice()).chain(|| {
            (
                ErrorKind::DeserializationError,
                "Unable to decode HD key bytes",
            )
        })?;

        let index = match address_type {
            AddressType::Transfer => hd_key.transfer_index,
            AddressType::Staking => hd_key.staking_index,
        };

        hd_key
            .seed
            .derive_key_pair(get_network(), address_type, index)
    }

    /// Clears all storage
    #[inline]
    pub fn clear(&self) -> Result<()> {
        self.storage.clear(KEYSPACE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::{DefaultWalletClient, WalletClient};

    use client_common::storage::MemoryStorage;

    #[test]
    fn check_hd_key_encode_decode() {
        let hd_key = HdKey {
            staking_index: 0,
            transfer_index: 0,
            seed: HDSeed::new(vec![
                5, 60, 53, 84, 12, 242, 183, 58, 174, 139, 134, 77, 28, 50, 203, 135, 181, 100,
                155, 234, 4, 110, 57, 243, 155, 154, 44, 159, 112, 255, 130, 44, 171, 107, 46, 195,
                115, 216, 81, 144, 7, 21, 109, 237, 40, 136, 91, 227, 27, 77, 94, 2, 39, 164, 114,
                51, 145, 97, 19, 147, 4, 127, 154, 228,
            ]),
        };

        let encoded = hd_key.encode();
        assert_eq!(
            encoded,
            vec![
                0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 5, 60, 53, 84, 12, 242, 183, 58, 174, 139, 134, 77,
                28, 50, 203, 135, 181, 100, 155, 234, 4, 110, 57, 243, 155, 154, 44, 159, 112, 255,
                130, 44, 171, 107, 46, 195, 115, 216, 81, 144, 7, 21, 109, 237, 40, 136, 91, 227,
                27, 77, 94, 2, 39, 164, 114, 51, 145, 97, 19, 147, 4, 127, 154, 228,
            ],
            "encode should be backward-compatible"
        );

        let decoded_hd_key = HdKey::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(hd_key, decoded_hd_key);
    }

    #[test]
    fn check_deterministic_hdkey_staking() {
        let storage = MemoryStorage::default();
        let passphrase = SecUtf8::from("passphrase");
        let name = "testhdwallet";
        let mnemonic =
            Mnemonic::from_secstr(&SecUtf8::from("speed tortoise kiwi forward extend baby acoustic foil coach castle ship purchase unlock base hip erode tag keen present vibrant oyster cotton write fetch")).unwrap();

        let wallet = DefaultWalletClient::new_read_only(storage.clone());
        wallet
            .restore_wallet(&name, &passphrase, &mnemonic)
            .expect("restore wallet");

        assert!(
            wallet
                .new_staking_address(&name, &passphrase)
                .expect("get new staking address")
                .to_string()
                == "0x83fe11feb0887183eb62c30994bdd9e303497e3d"
        );

        assert!(
            wallet
                .new_staking_address(&name, &passphrase)
                .expect("get new staking address")
                .to_string()
                == "0xe5b4b42406a061752c78bf5c4d6d6fccca0b575f"
        );

        assert!(
            wallet
                .new_staking_address(&name, &passphrase)
                .expect("get new staking address")
                .to_string()
                == "0x7310a0328e446df02cb4fb668a7a6790cea8c96e"
        );

        assert!(
            wallet
                .new_staking_address(&name, &passphrase)
                .expect("get new staking address")
                .to_string()
                == "0x56cbf4a74f59dcf1e0064f0daff3b1cf177ea972"
        );
    }

    #[test]
    fn check_deterministic_hdkey_transfer() {
        let storage = MemoryStorage::default();
        let passphrase = SecUtf8::from("passphrase");
        let name = "testhdwallet";
        let mnemonic =
            Mnemonic::from_secstr(&SecUtf8::from("speed tortoise kiwi forward extend baby acoustic foil coach castle ship purchase unlock base hip erode tag keen present vibrant oyster cotton write fetch")).unwrap();

        let wallet = DefaultWalletClient::new_read_only(storage.clone());
        wallet
            .restore_wallet(&name, &passphrase, &mnemonic)
            .expect("restore wallet");

        assert!(
            wallet
                .new_transfer_address(&name, &passphrase)
                .expect("get new transfer address")
                .to_string()
                == "dcro13z2xw689qhpmv7ge9xg428ljg4848rtu5dcpdmxy3m6njdsjtd3sl30d8n"
        );

        assert!(
            wallet
                .new_transfer_address(&name, &passphrase)
                .expect("get new transfer address")
                .to_string()
                == "dcro1fnjq70pf9hvd2tkd3rj7pash6ph7p42qakqt2k39sjqp4m4p25kqclslnt"
        );

        assert!(
            wallet
                .new_transfer_address(&name, &passphrase)
                .expect("get new transfer address")
                .to_string()
                == "dcro1ee3exuxyv5pauameswxureamlvmptjm8tsg4lcwqpx2nclshc6eqt8fanm"
        );

        assert!(
            wallet
                .new_transfer_address(&name, &passphrase)
                .expect("get new transfer address")
                .to_string()
                == "dcro1kl06wz2ytp02zlneqzsmtaecxvqdelkgrp693xk55tj7zs5vns7sjheun0"
        );
    }
}
