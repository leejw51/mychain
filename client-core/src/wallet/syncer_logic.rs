use non_empty_vec::NonEmpty;
use std::collections::HashMap;
use thiserror::Error;

use chain_core::init::coin::{sum_coins, Coin, CoinError};
use chain_core::tx::{
    data::{
        input::{TxoIndex, TxoPointer},
        output::TxOut,
        TxId,
    },
    TransactionId,
};
use client_common::tendermint::types::Time;
use client_common::Transaction;

use super::syncer::FilteredBlock;
use crate::service::{Wallet, WalletState};
use crate::types::{BalanceChange, TransactionChange, TransactionInput, TransactionType};
use crate::WalletStateMemento;

#[derive(Error, Debug)]
pub(crate) enum SyncerLogicError {
    #[error("Total input amount exceeds maximum allowed value(txid: {0})")]
    TotalInputOutOfBound(String),
    #[error("Total output amount exceeds maximum allowed value(txid: {0})")]
    TotalOutputOutOfBound(String),
    #[error("Input index is invalid(txid: {0}, index: {1})")]
    InputIndexInvalid(String, TxoIndex),
    #[error("Inputs come from multiple wallets(txid: {0})")]
    InputFromMultipleWallets(String),
    #[error("Output amount is greater than input amount(txid: {0})")]
    OutputGreaterThanInput(String),
}

/// Update wallet state with batch blocks
pub(crate) fn handle_blocks(
    wallet: &Wallet,
    wallet_state: &WalletState,
    blocks: &[FilteredBlock],
    enclave_transactions: &[Transaction],
) -> Result<WalletStateMemento, SyncerLogicError> {
    let enclave_transactions = enclave_transactions
        .iter()
        .map(|tx| (tx.id(), tx))
        .collect::<HashMap<_, _>>();
    let mut memento = WalletStateMemento::default();

    for block in blocks {
        for tx in block.staking_transactions.iter() {
            if block.valid_transaction_ids.contains(&tx.id()) {
                handle_transaction(
                    wallet,
                    wallet_state,
                    &mut memento,
                    tx,
                    block.block_height,
                    block.block_time,
                )?;
            }
        }

        for txid in block.enclave_transaction_ids.iter() {
            if let Some(tx) = enclave_transactions.get(txid) {
                handle_transaction(
                    wallet,
                    wallet_state,
                    &mut memento,
                    tx,
                    block.block_height,
                    block.block_time,
                )?;
            }
        }
    }
    Ok(memento)
}

/// Update WalletStateMemento with transaction
pub(crate) fn handle_transaction(
    wallet: &Wallet,
    wallet_state: &WalletState,
    memento: &mut WalletStateMemento,
    transaction: &Transaction,
    block_height: u64,
    block_time: Time,
) -> Result<(), SyncerLogicError> {
    let transaction_id = transaction.id();
    let outputs = transaction.outputs().to_vec();
    let transaction_type = TransactionType::from(transaction);
    let inputs = decorate_inputs(wallet_state, transaction.inputs(), &transaction_id)?;
    let balance_change = calculate_balance_change(wallet, &transaction_id, &inputs, &outputs)?;

    let transaction_change = TransactionChange {
        transaction_id,
        inputs,
        outputs,
        balance_change,
        transaction_type,
        block_height,
        block_time,
    };

    on_transaction_change(wallet, memento, transaction_change);
    Ok(())
}

fn on_transaction_change(
    wallet: &Wallet,
    memento: &mut WalletStateMemento,
    transaction_change: TransactionChange,
) {
    for input in transaction_change.inputs.iter() {
        memento.remove_unspent_transaction(input.pointer.clone());
    }

    let transfer_addresses = wallet.transfer_addresses();

    for (i, output) in transaction_change.outputs.iter().enumerate() {
        // Only add unspent transaction if output address belongs to current wallet
        if transfer_addresses.contains(&output.address) {
            memento.add_unspent_transaction(
                TxoPointer::new(transaction_change.transaction_id, i),
                output.clone(),
            );
        }
    }

    memento.add_transaction_change(transaction_change);
}

fn decorate_inputs(
    wallet_state: &WalletState,
    raw_inputs: &[TxoPointer],
    txid: &TxId,
) -> Result<Vec<TransactionInput>, SyncerLogicError> {
    raw_inputs
        .iter()
        .map(|raw_input| {
            Ok(TransactionInput {
                output: wallet_state.get_output(raw_input).map_err(|_| {
                    SyncerLogicError::InputIndexInvalid(hex::encode(txid), raw_input.index)
                })?,
                pointer: raw_input.clone(),
            })
        })
        .collect()
}

fn sum_outputs<'a>(outputs: impl Iterator<Item = &'a TxOut>) -> Result<Coin, CoinError> {
    sum_coins(outputs.map(|output| output.value))
}

fn calculate_balance_change<'a>(
    wallet: &'a Wallet,
    transaction_id: &'a TxId,
    inputs: &'a [TransactionInput],
    outputs: &'a [TxOut],
) -> Result<BalanceChange, SyncerLogicError> {
    let encode_txid = || hex::encode(&transaction_id);

    let transfer_addresses = wallet.transfer_addresses();
    let is_our_address = |addr| transfer_addresses.contains(addr);
    let our_output = |input: &'a TransactionInput| -> Option<&'a TxOut> {
        input.output.as_ref().and_then(|output| {
            if is_our_address(&output.address) {
                Some(output)
            } else {
                None
            }
        })
    };

    // Either all spent output is ours (outgoing), or none of it is (incoming).
    let spent_outputs: Option<NonEmpty<&TxOut>> = inputs
        .iter()
        .map(|input| our_output(input))
        .collect::<Option<Vec<_>>>()
        .and_then(NonEmpty::new);

    let total_output = sum_outputs(outputs.iter())
        .map_err(|_| SyncerLogicError::TotalOutputOutOfBound(encode_txid()))?;

    let total_output_ours = sum_outputs(
        outputs
            .iter()
            .filter(|output| is_our_address(&output.address)),
    )
    .map_err(|_| SyncerLogicError::TotalOutputOutOfBound(encode_txid()))?;

    match spent_outputs {
        None => {
            // If one of the spent outputs is others, then all of it needs to be others.
            if inputs.iter().any(|input| our_output(input).is_some()) {
                Err(SyncerLogicError::InputFromMultipleWallets(encode_txid()))
            } else {
                Ok(if total_output_ours == Coin::zero() {
                    BalanceChange::NoChange
                } else {
                    BalanceChange::Incoming {
                        value: total_output_ours,
                    }
                })
            }
        }
        Some(spent_outputs) => {
            let total_input = sum_outputs(spent_outputs.iter().cloned())
                .map_err(|_| SyncerLogicError::TotalInputOutOfBound(encode_txid()))?;
            let fee = (total_input - total_output)
                .map_err(|_| SyncerLogicError::OutputGreaterThanInput(encode_txid()))?;
            // (total_input - fee) - total_output_ours
            // panic is impossible because total_output_ours is subset of total_output
            let value = (total_output - total_output_ours).expect("impossible");
            Ok(BalanceChange::Outgoing { fee, value })
        }
    }
}

#[cfg(test)]
mod tests {
    use secstr::SecUtf8;
    use std::str::FromStr;

    use chain_core::init::{address::RedeemAddress, coin::Coin};
    use chain_core::state::account::{StakedStateAddress, StakedStateOpAttributes, UnbondTx};
    use chain_core::tx::data::{address::ExtendedAddr, attribute::TxAttributes, output::TxOut, Tx};
    use chain_core::tx::TransactionId;
    use chain_tx_filter::BlockFilter;
    use client_common::{storage::MemoryStorage, PublicKey, Result, Transaction};

    use super::*;
    use crate::service::load_wallet;
    use crate::types::WalletKind;
    use crate::wallet::{DefaultWalletClient, WalletClient};

    fn create_test_wallet(n: usize) -> Result<Vec<Wallet>> {
        let storage = MemoryStorage::default();
        let passphrase = &SecUtf8::from("passphrase");
        let wallet = DefaultWalletClient::new_read_only(storage.clone());

        (0..n)
            .map(|i| {
                let name = format!("name{}", i);
                wallet
                    .new_wallet(&name, passphrase, WalletKind::Basic)
                    .expect("new wallet");
                wallet
                    .new_transfer_address(&name, passphrase)
                    .expect("new transfer address");
                Ok(load_wallet(&storage, &name, &passphrase)?.unwrap())
            })
            .collect()
    }

    fn transfer_transaction() -> Transaction {
        Transaction::TransferTransaction(Tx::new_with(
            Vec::new(),
            vec![TxOut::new(
                ExtendedAddr::OrTree([0; 32]),
                Coin::new(100).unwrap(),
            )],
            TxAttributes::default(),
        ))
    }

    fn unbond_transaction() -> Transaction {
        let addr = StakedStateAddress::from(
            RedeemAddress::from_str("0x0e7c045110b8dbf29765047380898919c5cb56f4").unwrap(),
        );
        Transaction::UnbondStakeTransaction(UnbondTx::new(
            addr,
            0,
            Coin::new(100).unwrap(),
            StakedStateOpAttributes::new(0),
        ))
    }

    fn block_header(
        view_keys: &[PublicKey],
        enclave_txs: &[Transaction],
        other_txs: &[Transaction],
    ) -> FilteredBlock {
        let mut block_filter = BlockFilter::default();
        for view_key in view_keys {
            block_filter.add_view_key(&view_key.into());
        }

        let valid_transaction_ids = enclave_txs
            .iter()
            .map(|tx| tx.id())
            .chain(other_txs.iter().map(|tx| tx.id()))
            .collect();
        FilteredBlock {
            app_hash: "3891040F29C6A56A5E36B17DCA6992D8F91D1EAAB4439D008D19A9D703271D3C".to_owned(),
            block_height: 1,
            block_time: Time::from_str("2019-04-09T09:38:41.735577Z").unwrap(),
            valid_transaction_ids,
            enclave_transaction_ids: enclave_txs.iter().map(|tx| tx.id()).collect(),
            block_filter,
            staking_transactions: other_txs.to_vec(),
        }
    }

    #[test]
    fn check_syncer_logic_basic() {
        let wallets = create_test_wallet(1).unwrap();
        let view_keys = wallets
            .iter()
            .map(|wallet| wallet.view_key.clone())
            .collect::<Vec<_>>();
        let mut state = WalletState::default();
        let tx = transfer_transaction();
        let tx_cloned = tx.clone();
        let blocks = [block_header(
            &view_keys,
            &[tx.clone()],
            &[unbond_transaction()],
        )];
        let memento = handle_blocks(&wallets[0], &state, &blocks, &[tx.clone()]).unwrap();
        state.apply_memento(&memento).expect("apply memento");
        assert_eq!(
            state.transaction_history.iter().next().unwrap().0,
            &tx_cloned.id()
        );
    }

    fn transfer_transactions(addresses: [ExtendedAddr; 2]) -> [Transaction; 2] {
        let transaction1 = Transaction::TransferTransaction(Tx::new_with(
            Vec::new(),
            vec![TxOut::new(addresses[0].clone(), Coin::new(100).unwrap())],
            TxAttributes::default(),
        ));

        let transaction2 = Transaction::TransferTransaction(Tx::new_with(
            vec![TxoPointer::new(transaction1.id(), 0)],
            vec![TxOut::new(addresses[1].clone(), Coin::new(100).unwrap())],
            TxAttributes::default(),
        ));

        [transaction1, transaction2]
    }

    #[test]
    fn check_syncer_logic_tx_flow() {
        let wallets = create_test_wallet(2).unwrap();
        let view_keys = wallets
            .iter()
            .map(|wallet| wallet.view_key.clone())
            .collect::<Vec<_>>();
        let address1 = wallets[0].transfer_addresses().into_iter().next().unwrap();
        let address2 = wallets[1].transfer_addresses().into_iter().next().unwrap();
        let transactions = transfer_transactions([address1.clone(), address2.clone()]);
        let mut states = wallets
            .iter()
            .map(|_| WalletState::default())
            .collect::<Vec<_>>();

        let txs = [transactions[0].clone()];
        let blocks = [block_header(&[view_keys[0].clone()], &txs, &[])];
        {
            let memento = handle_blocks(&wallets[0], &states[0], &blocks, &txs)
                .expect("handle block for wallet1");
            states[0].apply_memento(&memento).expect("apply memento1");
        }
        {
            let memento = handle_blocks(&wallets[1], &states[1], &blocks, &[])
                .expect("handle block for wallet2");
            states[1].apply_memento(&memento).expect("apply memento2");
        }
        assert_eq!(states[0].balance, Coin::new(100).unwrap());
        assert_eq!(states[0].transaction_history.len(), 1);
        assert_eq!(states[0].unspent_transactions.len(), 1);

        let txs = [transactions[1].clone()];
        let blocks = [block_header(&view_keys, &txs, &[])];

        {
            let memento = handle_blocks(&wallets[0], &states[0], &blocks, &txs)
                .expect("handle block for wallet1");
            states[0].apply_memento(&memento).expect("apply memento1");
        }

        {
            let memento = handle_blocks(&wallets[1], &states[1], &blocks, &txs)
                .expect("handle block for wallet2");
            states[1].apply_memento(&memento).expect("apply memento2");
        }

        assert_eq!(states[0].balance, Coin::new(0).unwrap());
        assert_eq!(states[0].transaction_history.len(), 2);
        assert_eq!(states[0].unspent_transactions.len(), 0);

        assert_eq!(states[1].balance, Coin::new(100).unwrap());
        assert_eq!(states[1].transaction_history.len(), 1);
        assert_eq!(states[1].unspent_transactions.len(), 1);
    }
}
