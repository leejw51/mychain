use super::ChainNodeApp;
use crate::enclave_bridge::EnclaveProxy;
use crate::storage::tx::verify;
use abci::*;
use chain_core::state::account::StakedState;
use chain_core::tx::fee::Fee;
use chain_core::tx::TxAux;
use chain_tx_validation::ChainInfo;
use parity_scale_codec::Decode;

/// Wrapper to astract over CheckTx and DeliverTx requests
pub trait RequestWithTx {
    fn tx(&self) -> &[u8];
}

impl RequestWithTx for RequestCheckTx {
    fn tx(&self) -> &[u8] {
        &self.tx[..]
    }
}

impl RequestWithTx for RequestDeliverTx {
    fn tx(&self) -> &[u8] {
        &self.tx[..]
    }
}

/// Wrapper to astract over CheckTx and DeliverTx responses
pub trait ResponseWithCodeAndLog {
    fn set_code(&mut self, _: u32);
    fn add_log(&mut self, _: &str);
}

impl ResponseWithCodeAndLog for ResponseCheckTx {
    fn set_code(&mut self, new_code: u32) {
        self.code = new_code;
    }

    fn add_log(&mut self, entry: &str) {
        self.log += entry;
    }
}

impl ResponseWithCodeAndLog for ResponseDeliverTx {
    fn set_code(&mut self, new_code: u32) {
        self.code = new_code;
    }

    fn add_log(&mut self, entry: &str) {
        self.log += entry;
    }
}

impl<T: EnclaveProxy> ChainNodeApp<T> {
    /// Gets CheckTx or DeliverTx requests, tries to parse its data into TxAux and validate that TxAux.
    /// Returns Some(parsed txaux, (paid fee, updated staking account)) if OK, or None if some problems (and sets log + error code in the passed in response).
    pub fn validate_tx_req(
        &mut self,
        _req: &dyn RequestWithTx,
        resp: &mut dyn ResponseWithCodeAndLog,
    ) -> Option<(TxAux, (Fee, Option<StakedState>))> {
        let data = Vec::from(_req.tx());
        let dtx = TxAux::decode(&mut data.as_slice());
        match dtx {
            Err(e) => {
                resp.set_code(1);
                resp.add_log(&format!("failed to deserialize tx: {}", e.what()));
                None
            }
            Ok(txaux) => {
                let state = self.last_state.as_ref().expect("the app state is expected");
                let min_fee = state
                    .network_params
                    .calculate_fee(_req.tx().len())
                    .expect("invalid fee policy");
                let fee_paid = verify(
                    &mut self.tx_validator,
                    &txaux,
                    ChainInfo {
                        min_fee_computed: min_fee,
                        chain_hex_id: self.chain_hex_id,
                        previous_block_time: state.block_time,
                        unbonding_period: state.network_params.get_unbonding_period(),
                    },
                    &self.uncommitted_account_root_hash,
                    self.storage.db.clone(),
                    &self.accounts,
                );
                match fee_paid {
                    Ok(fee) => {
                        resp.set_code(0);
                        Some((txaux, fee))
                    }
                    Err(fee_err) => {
                        resp.set_code(1);
                        resp.add_log(&format!("verification failed: {}", fee_err));
                        None
                    }
                }
            }
        }
    }
}
