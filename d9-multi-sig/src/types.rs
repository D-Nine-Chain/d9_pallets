use crate::pallet::Config;
use crate::pallet::Pallet;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use frame_system::pallet_prelude::*;
use sp_core::blake2_256;
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
pub struct MultiSignatureAccount<T: Config> {
    /// the address of this multi signature account
    pub address: T::AccountId,
    /// the 'admins' of this multi signature account. if None then any of the signatories can propose a transaction
    pub transaction_proposers: Option<BoundedVec<T::AccountId, T::MaxSignatories>>,
    /// the possible signers for this multi signature account
    pub signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
    pub pending_transaction: Option<BoundedVec<u8, T::MaxTransactionSize>>,
    //note need a place to hold pending transactions (approvals)
    pub minimum_signatories: u32,
}

impl<T: Config> MultiSignatureAccount<T> {
    pub fn new(
        mut signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
        minimum_signatories: u32,
    ) -> Self {
        assert!(
            (1 <= minimum_signatories) && (minimum_signatories <= signatories.len() as u32),
            "minimum signatories must be between 1 and the number of signatories"
        );
        signatories.sort();
        let address = Self::construct_address(&signatories);
        Self {
            address,
            transaction_proposers: None,
            signatories,
            pending_transaction: None,
            minimum_signatories,
        }
    }

    pub fn add_proposer(
        &mut self,
        proposer: &T::AccountId,
    ) -> Result<(), MultiSignatureAccountError> {
        self.validate_new_proposer(proposer)?;
        if let Some(ref mut transaction_proposers) = self.transaction_proposers {
            let push_result = transaction_proposers.try_push(proposer.clone());

            if push_result.is_err() {
                return Err(MultiSignatureAccountError::AlreadyAtMaxProposers);
            }
        } else {
            let mut proposers = BoundedVec::<T::AccountId, T::MaxSignatories>::new();
            let _ = proposers.try_push(proposer.clone());
            self.transaction_proposers = Some(proposers);
        }
        Ok(())
    }

    pub fn is_signatory(&self, signatory: &T::AccountId) -> bool {
        self.signatories.contains(signatory)
    }

    fn construct_address(
        signatories: &BoundedVec<T::AccountId, T::MaxSignatories>,
    ) -> T::AccountId {
        let entropy = (b"d9-multi-sig:v1", signatories).using_encoded(blake2_256);
        Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
            .expect("infinite length input; no invalid inputs for type; qed")
    }

    fn validate_new_proposer(
        &self,
        proposer: &T::AccountId,
    ) -> Result<(), MultiSignatureAccountError> {
        if let Some(ref transaction_proposers) = self.transaction_proposers {
            if transaction_proposers.contains(proposer) {
                return Err(MultiSignatureAccountError::AccountAlreadyProposer);
            }
            if (transaction_proposers.len() + 1) > self.signatories.len() {
                return Err(MultiSignatureAccountError::AlreadyAtMaxProposers);
            }
            if !self.is_signatory(proposer) {
                return Err(MultiSignatureAccountError::AccountNotSignatory);
            }
        }
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
pub enum MultiSignatureAccountError {
    MultiSignatureAccountError,
    MultiSignatureAccountAlreadyExists,
    AccountAlreadyProposer,
    AccountNotSignatory,
    AlreadyAtMaxProposers,
    MultiSignatureAccountNotFound,
    PendingTransactionAlreadyExists,
    SignatoriesOutOfOrder,
    SenderInSignatories,
}
