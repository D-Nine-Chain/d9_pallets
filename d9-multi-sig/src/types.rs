use crate::pallet::Config;
use crate::pallet::Pallet;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use sp_runtime::traits::Bounded;
use sp_runtime::traits::Convert;

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
pub struct MultiSignatureAccount<T: Config> {
    /// the address of this multi signature account
    pub address: T::AccountId,
    /// the 'admin' of this multi signature account. if None then any of the signatories can propose a transaction
    pub transaction_proposers: Option<BoundedVec<T::AccountId, T::MaxSignatories>>,
    pub signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
    pub pending_transaction: Option<BoundedVec<u8, T::MaxTransactionSize>>,
    //note need a place to hold pending transactions (approvals)
    pub minimum_signatories: u32,
}

impl<T: Config> MultiSignatureAccount<T> {
    pub fn new(
        signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
        minimum_signatories: u32,
    ) -> Self {
        assert!(
            (0 < minimum_signatories) && (minimum_signatories <= signatories.len() as u32),
            "Number of signatories must be greater than or equal to minimum signatories"
        );
        Self {
            address: T::AccountId::default(),
            transaction_proposers: None,
            signatories,
            pending_transaction: None,
            minimum_signatories,
        }
    }

    pub fn add_proposer(&mut self, proposer: T::AccountId) {}

    pub fn is_signatory(&self, signatory: &T::AccountId) -> bool {
        self.signatories.contains(signatory)
    }

    fn construct_address() -> T::AccountId {}
}

pub enum MultiSignatureAccountErrors {
    MultiSignatureAccountError,
    MultiSignatureAccountAlreadyExists,
    MultiSignatureAccountNotFound,
    PendingTransactionAlreadyExists,
}
