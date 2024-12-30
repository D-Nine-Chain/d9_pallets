use super::PendingCall;
use crate::pallet::Config;
use crate::pallet::Error;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{pallet_prelude::*, BoundedVec};
use sp_core::blake2_256;
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
    /// if `None` then all signatories are authors
    pub authors: Option<BoundedVec<T::AccountId, T::MaxSignatories>>,
    /// the possible signers for this multi signature account T::MaxSignatories
    pub signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
    /// minimum number of signatories required to sign a transaction
    pub minimum_signatories: u32,
    /// pending calls max of T::MaxPendingTransactions
    pub pending_calls: BoundedVec<PendingCall<T>, T::MaxPendingCalls>,
}

impl<T: Config> MultiSignatureAccount<T> {
    pub fn new(
        signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
        authors: Option<BoundedVec<T::AccountId, T::MaxSignatories>>,
        min_approvals: u32,
    ) -> Result<Self, MultiSigAcctError> {
        let address = Self::construct_address(&signatories);
        Ok(Self {
            address,
            authors,
            signatories,
            minimum_signatories: min_approvals,
            pending_calls: BoundedVec::default(),
        })
    }

    pub fn add_authors(&mut self, authors: &[T::AccountId]) -> Result<(), MultiSigAcctError> {
        self.validate_authors_list(authors)?;
        match &mut self.authors {
            Some(authors_bounded_vec) => {
                let result = authors_bounded_vec.try_extend(authors.iter().cloned());
                if result.is_err() {
                    return Err(MultiSigAcctError::AuthorVecTooLong);
                }
            }
            None => {
                let bounded_authors_res = BoundedVec::try_from(authors.to_vec());
                if bounded_authors_res.is_err() {
                    return Err(MultiSigAcctError::AuthorVecTooLong);
                }
                self.authors = Some(bounded_authors_res.unwrap());
            }
        }
        Ok(())
    }

    /// add a call to `pending_calls`
    pub fn add_call(&mut self, call: PendingCall<T>) -> Result<(), MultiSigAcctError> {
        self.pending_calls
            .try_push(call)
            .map_err(|_| MultiSigAcctError::AtPendingCallLimit)
    }

    /// remove a call from `pending_calls`
    pub fn remove_call(&mut self, call_id: [u8; 32]) -> Result<(), MultiSigAcctError> {
        let index = self
            .pending_calls
            .iter()
            .position(|call| call.id == call_id)
            .ok_or(MultiSigAcctError::CallNotFound)?;
        self.pending_calls.remove(index);
        Ok(())
    }

    /// is the account_id a signatory of this multi sig account
    pub fn is_signatory(&self, account_id: &T::AccountId) -> bool {
        // can be done as signatories.as_slice().binary_search(&account_id).is_ok()
        // keep as .contains just to be extra safe just in case this receives an unordered signatories
        self.signatories.contains(account_id)
    }

    pub fn is_author(&self, account_id: &T::AccountId) -> bool {
        match &self.authors {
            Some(authors) => authors.contains(account_id),
            None => self.signatories.contains(account_id),
        }
    }

    pub fn adjust_min_approvals(
        &mut self,
        new_min_approvals: u32,
    ) -> Result<(), MultiSigAcctError> {
        if !(2..=self.signatories.len() as u32).contains(&new_min_approvals) {
            return Err(MultiSigAcctError::MinApprovalsOutOfRange);
        }
        self.minimum_signatories = new_min_approvals;
        Ok(())
    }
    /// deterministically construct the address of a multi sig account
    ///
    /// signatories is ordered; the same set begets same address
    fn construct_address(
        signatories: &BoundedVec<T::AccountId, T::MaxSignatories>,
    ) -> T::AccountId {
        let entropy = (b"d9-multi-sig:v1", signatories).using_encoded(blake2_256);
        Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
            .expect("infinite length input; no invalid inputs for type; qed")
    }

    fn validate_authors_list(&self, new_authors: &[T::AccountId]) -> Result<(), MultiSigAcctError> {
        let current_authors_len = self.authors.as_ref().map(|p| p.len()).unwrap_or(0);
        let open_author_slots = self.signatories.len().saturating_sub(current_authors_len);

        if new_authors.len() >= open_author_slots {
            return Err(MultiSigAcctError::AuthorVecTooLong);
        }
        let existing_authors = self.authors.as_ref().map(|p| p.as_slice()).unwrap_or(&[]);
        for author in new_authors {
            if existing_authors.contains(author) {
                return Err(MultiSigAcctError::AccountAlreadyAuthor);
            }
            if !self.is_signatory(author) {
                return Err(MultiSigAcctError::AccountNotSignatory);
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

pub enum MultiSigAcctError {
    /// minimum of 2 signatories required
    SignatoriesListTooShort,
    /// account already in author list
    AccountAlreadyAuthor,
    /// proposer length too long
    AuthorVecTooLong,
    /// not part of the signatories of multi sig account so can not be proposer or sign
    AccountNotSignatory,
    /// authors is at T::MaxSignatories - 1
    AtMaxAuthors,
    /// pending call  limit defined by T::MaxPendingTransactions
    AtPendingCallLimit,
    /// when adjusting min approvals the new value must be between 2 and the number of signatories
    MinApprovalsOutOfRange,
    CallNotFound,
}

impl<T> From<MultiSigAcctError> for Error<T> {
    fn from(account_error: MultiSigAcctError) -> Self {
        match account_error {
            MultiSigAcctError::SignatoriesListTooShort => Error::<T>::SignatoriesTooShort,
            MultiSigAcctError::AccountAlreadyAuthor => Error::<T>::AccountAlreadyAuthor,
            MultiSigAcctError::AuthorVecTooLong => Error::<T>::AuthorVecTooLong,
            MultiSigAcctError::AccountNotSignatory => Error::<T>::AccountNotAuthor,
            MultiSigAcctError::AtMaxAuthors => Error::<T>::AccountErrorMaxAuthors,
            MultiSigAcctError::AtPendingCallLimit => Error::<T>::CallLimit,
            MultiSigAcctError::MinApprovalsOutOfRange => Error::<T>::MinApprovalOutOfRange,
            MultiSigAcctError::CallNotFound => Error::<T>::CallNotFound,
        }
    }
}
