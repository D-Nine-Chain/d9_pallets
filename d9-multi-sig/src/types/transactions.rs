use crate::pallet::Config;
use crate::pallet::Pallet;
use codec::MaxEncodedLen;
use frame_support::storage::bounded_btree_set::BoundedBTreeSet;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use frame_system::pallet_prelude::*;
use sp_core::blake2_256;
use sp_core::bounded;
use sp_runtime::traits::Bounded;
use sp_runtime::traits::Convert;
use sp_runtime::traits::TrailingZeroInput;
#[derive(
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct PendingCalls<T: Config> {
    pub call: Box<<T as Config>::RuntimeCall>,
    pub approvals: BoundedBTreeSet<T::AccountId, T::MaxSignatories>,
}

impl<T: Config> PendingCalls<T> {
    pub fn new(call: Box<<T as Config>::RuntimeCall>, author: T::AccountId) -> Self {
        let mut approvals = BoundedBTreeSet::new();
        let _ = approvals.try_insert(author);
        Self { call, approvals }
    }

    pub fn add_approval(&mut self, author: T::AccountId) -> Result<(), PendingCallError> {
        let result = self.approvals.try_insert(author);
        Ok(())
    }

    pub fn remove_approval(&mut self, author: T::AccountId) -> Result<(), PendingCallError> {
        let result = self.approvals.remove(&author);
        Ok(())
    }
}

#[derive(
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub enum PendingCallError {
    ExecutionError,
}
