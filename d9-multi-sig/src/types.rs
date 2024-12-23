use crate::pallet::Config;
use crate::pallet::Pallet;
use codec::MaxEncodedLen;
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
pub struct MultiSignatureAccount<T: Config> {
    /// the address of this multi signature account
    pub address: T::AccountId,
    /// the 'admins' of this multi signature account. if None then any of the signatories can propose a transaction or cancel an existing transaction pending signatures
    pub authors: Option<BoundedVec<T::AccountId, T::MaxSignatories>>,
    /// the possible signers for this multi signature account
    pub signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
    pub pending_transaction: Option<BoundedVec<u8, T::MaxTransactionSize>>,
    //note need a place to hold pending transactions (approvals)
    pub minimum_signatories: u32,
}

impl<T: Config> MultiSignatureAccount<T> {
    pub fn new(
        mut signatories: BoundedVec<T::AccountId, T::MaxSignatories>,
        authors_opt: Option<Vec<T::AccountId>>,
        minimum_signatories: u32,
    ) -> Result<Self, MultiSigAcctError<T>> {
        // Ensure 1 <= minimum_signatories <= signatories.len()
        if minimum_signatories < 1 || minimum_signatories > signatories.len() as u32 {
            return Err(MultiSigAcctError::MinimumSignatoriesRangeError);
        }

        // Sort signatories for consistent ordering
        signatories.sort();
        let address = Self::construct_address(&signatories);

        // Convert Option<Vec<...>> -> Option<BoundedVec<...>> or None
        let authors_opt = match authors_opt {
            Some(mut authors) => {
                // If authors is the same size as signatories, force authors to None
                if authors.len() == signatories.len() {
                    None
                } else {
                    // Sort authors to detect duplicates easily
                    authors.sort();

                    // Check for duplicates by ensuring no adjacent pair is the same
                    let no_duplicates = authors.windows(2).all(|pair| pair[0] != pair[1]);
                    if !no_duplicates {
                        return Err(MultiSigAcctError::DuplicateAuthors);
                    }

                    // Check that every author is actually a signatory
                    for author in &authors {
                        if !signatories.contains(author) {
                            return Err(MultiSigAcctError::AccountNotSignatory(author.clone()));
                        }
                    }
                    // Convert Vec -> BoundedVec via `.into()`
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
            pending_transaction: None,
            minimum_signatories,
        })
    }

    pub fn add_authors(&mut self, authors: &[T::AccountId]) -> Result<(), MultiSigAcctError<T>> {
        self.validate_authors(authors)?;
        match &mut self.authors {
            Some(authors_bounded_vec) => {
                let result = authors_bounded_vec.try_extend(authors.iter().cloned());
                if result.is_err() {
                    return Err(MultiSigAcctError::AtMaxAuthors);
                    //note give this a better error name
                }
            }
            None => {
                let bounded_authors_res = BoundedVec::try_from(authors.to_vec());
                if bounded_authors_res.is_err() {
                    return Err(MultiSigAcctError::AtMaxAuthors);
                }
                self.authors = Some(bounded_authors_res.unwrap());
            }
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

    fn validate_authors(&self, authors: &[T::AccountId]) -> Result<(), MultiSigAcctError<T>> {
        let current_authors_len = self.authors.as_ref().map(|p| p.len()).unwrap_or(0);
        let max_new_authors = self.signatories.len().saturating_sub(current_authors_len);

        if authors.len() >= max_new_authors {
            return Err(MultiSigAcctError::AuthorVecTooLong);
        }
        let existing_authors = self.authors.as_ref().map(|p| p.as_slice()).unwrap_or(&[]);
        for author in authors {
            if existing_authors.contains(author) {
                return Err(MultiSigAcctError::AlreadyAuthor(author.clone()));
            }
            if !self.is_signatory(author) {}
            return Err(MultiSigAcctError::AccountNotSignatory(author.clone()));
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
#[scale_info(skip_type_params(T))]
pub enum MultiSigAcctError<T: Config> {
    /// minimum signatories must be between 1 and the number of signatories
    MinimumSignatoriesRangeError,
    /// duplicates in Authors list
    DuplicateAuthors,
    /// a multi sig account with the **exact** same signatories already exists.
    MultiSigAccountExists(T::AccountId),
    /// proposed account already in the list
    AlreadyAuthor(T::AccountId),
    /// proposer length too long
    AuthorVecTooLong,
    /// not part of the signatories of multi sig account so can not be proposer or sign
    AccountNotSignatory(T::AccountId),
    AtMaxAuthors,
    MultiSigAccountNotFound,
    AlreadyPendingTransaction,
    SignatoriesOutOfOrder,
    SenderInSignatories(T::AccountId),
}
