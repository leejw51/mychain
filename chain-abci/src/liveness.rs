use bit_vec::BitVec;
use parity_scale_codec::{Decode, Encode, Error, Input, Output};
use serde::{Deserialize, Serialize};

use chain_core::state::tendermint::BlockHeight;

/// Liveness tracker for a validator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LivenessTracker {
    /// Holds data to measure liveness
    ///
    /// # Note
    ///
    /// - Size of this `BitVec` should be equal to `block_signing_window` in jailing parameters in genesis.
    /// - Stores `true` at `index = height % block_signing_window`, if validator has signed that block, `false`
    ///   otherwise.
    liveness: BitVec,
}

impl LivenessTracker {
    /// Creates a new instance of liveness tracker
    #[inline]
    pub fn new(block_signing_window: u16) -> Self {
        Self {
            liveness: BitVec::from_elem(block_signing_window as usize, true),
        }
    }

    /// Updates liveness tracker with new block data
    pub fn update(&mut self, block_height: BlockHeight, signed: bool) {
        let block_signing_window = self.liveness.len();
        let update_index = (block_height as usize - 1) % block_signing_window; // Because `block_height` starts from 1
        self.liveness.set(update_index, signed)
    }

    /// Checks if validator is live or not
    #[inline]
    pub fn is_live(&self, missed_block_threshold: u16) -> bool {
        // FIXME: use POPCOUNT
        let zero_count = self.liveness.iter().filter(|x| !x).count();
        zero_count < missed_block_threshold as usize
    }
}

impl Encode for LivenessTracker {
    fn size_hint(&self) -> usize {
        std::mem::size_of::<u16>() + self.liveness.to_bytes().size_hint()
    }

    fn encode_to<W: Output>(&self, dest: &mut W) {
        (self.liveness.len() as u16).encode_to(dest);
        self.liveness.to_bytes().encode_to(dest);
    }
}

impl Decode for LivenessTracker {
    fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
        let length = u16::decode(input)?;
        let bytes = <Vec<u8>>::decode(input)?;

        let mut liveness = BitVec::from_bytes(&bytes);
        liveness.truncate(length as usize);

        Ok(LivenessTracker { liveness })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_liveness_tracker_encode_decode() {
        let mut initial = LivenessTracker::new(50);
        initial.update(1, true);
        initial.update(2, false);

        let encoded = initial.encode();
        let decoded = LivenessTracker::decode(&mut encoded.as_ref()).unwrap();

        assert_eq!(initial, decoded);
    }

    #[test]
    fn check_liveness_tracker() {
        let mut tracker = LivenessTracker::new(5);
        tracker.update(1, true);
        tracker.update(2, false);
        tracker.update(3, true);
        tracker.update(4, false);
        tracker.update(5, true);

        assert!(tracker.is_live(3));
        assert!(!tracker.is_live(2));
    }
}
