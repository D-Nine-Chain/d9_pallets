use super::PendingCall;
use crate::pallet::Config;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
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
        mut signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
        authors_opt: Option<Vec<T::AccountId>>,
        minimum_signatories: u32,
    ) -> Result<Self, MultiSigAcctError> {
        // Ensure 1 <= minimum_signatories <= signatories.len()
        if minimum_signatories < 1 || minimum_signatories > signatories.len() as u32 {
            return Err(MultiSigAcctError::MinimumSignatoriesRangeError);
        }
        // Sort signatories for consistent ordering
        signatories.sort();
        let address = Self::construct_address(&signatories);

        let authors_opt = match authors_opt {
            Some(mut authors) => {
                // If authors is the same size as signatories, force authors to None
                if authors.len() == signatories.len() {
                    None
                } else {
                    authors.sort();
                    let no_duplicates = authors.windows(2).all(|pair| pair[0] != pair[1]);
                    if !no_duplicates {
                        return Err(MultiSigAcctError::DuplicateAuthors);
                    }
                    for author in &authors {
                        if !signatories.contains(author) {
                            return Err(MultiSigAcctError::AccountNotSignatory);
                        }
                    }
                    let try_into_bounded_vec = BoundedVec::try_from(authors);
                    if try_into_bounded_vec.is_err() {
                        return Err(MultiSigAcctError::AtMaxAuthors);
                    }
                    Some(try_into_bounded_vec.unwrap())
                }
            }
            None => None,
        };

        Ok(Self {
            address,
            authors: authors_opt,
            signatories,
            minimum_signatories,
            pending_calls: BoundedVec::default(),
        })
    }

    pub fn add_authors(&mut self, authors: &[T::AccountId]) -> Result<(), MultiSigAcctError> {
        self.validate_authors(authors)?;
        match &mut self.authors {
            Some(authors_bounded_vec) => {
                let result = authors_bounded_vec.try_extend(authors.iter().cloned());
                if result.is_err() {
                    return Err(MultiSigAcctError::AuthorExtendError);
                }
            }
            None => {
                let bounded_authors_res = BoundedVec::try_from(authors.to_vec());
                if bounded_authors_res.is_err() {
                    return Err(MultiSigAcctError::AuthorExtendError);
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
            .map_err(|_| MultiSigAcctError::AtPendingTransactionLimit)
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
        self.signatories.contains(account_id)
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

    fn validate_authors(&self, authors: &[T::AccountId]) -> Result<(), MultiSigAcctError> {
        let current_authors_len = self.authors.as_ref().map(|p| p.len()).unwrap_or(0);
        let max_new_authors = self.signatories.len().saturating_sub(current_authors_len);

        if authors.len() >= max_new_authors {
            return Err(MultiSigAcctError::AuthorVecTooLong);
        }
        let existing_authors = self.authors.as_ref().map(|p| p.as_slice()).unwrap_or(&[]);
        for author in authors {
            if existing_authors.contains(author) {
                return Err(MultiSigAcctError::AlreadyAuthor);
            }
            if !self.is_signatory(author) {}
            return Err(MultiSigAcctError::AccountNotSignatory);
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
    /// minimum signatories must be between 1 and the number of signatories
    MinimumSignatoriesRangeError,
    /// duplicates in Authors list
    DuplicateAuthors,
    /// a multi sig account with the **exact** same signatories already exists.
    MultiSigAccountExists,
    /// proposed account already in the list
    AlreadyAuthor,
    /// proposer length too long
    AuthorVecTooLong,
    /// error in extending authors
    AuthorExtendError,
    /// not part of the signatories of multi sig account so can not be proposer or sign
    AccountNotSignatory,
    /// authors is at T::MaxSignatories - 1
    AtMaxAuthors,
    /// pending call  limit defined by T::MaxPendingTransactions
    AtPendingTransactionLimit,
    /// call is not in pending_calls
    CallNotFound,
}
