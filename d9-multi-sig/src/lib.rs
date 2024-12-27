#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
mod types;
use frame_support::BoundedVec;
pub use types::*;
pub type BoundedCallOf<T> = BoundedVec<u8, <T as Config>::MaxCallSize>;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::{
        DispatchResult, Dispatchable, GetDispatchInfo, PostDispatchInfo,
    };
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::collections::btree_set::BTreeSet;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxSignatories: Get<u32>;
        type MaxPendingCalls: Get<u32>;
        type MaxMultiSigsPerAccountId: Get<u32>;
        type RuntimeCall: Parameter
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
            + GetDispatchInfo
            + From<Call<Self>>;
        type MaxCallSize: Get<u32>; //note what is a good value forn max call size
    }

    /// the existent multi signature accounts
    ///
    /// key: multi signature account address value: multi signature account
    #[pallet::storage]
    #[pallet::getter(fn multi_signature_accounts)]
    pub type MultiSignatureAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, MultiSignatureAccount<T>, OptionQuery>;

    /// key: normal accounts value: vec of multi signature account ids that user is a member of
    #[pallet::storage]
    #[pallet::getter(fn user_multi_signature_accounts)]
    pub type UserMultiSigAccounts<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<T::AccountId, T::MaxMultiSigsPerAccountId>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// (creator, created)
        MultiSignatureAccountCreated(T::AccountId, T::AccountId),
        /// (updater, updated)
        MultiSignatureAccountUpdated(T::AccountId, T::AccountId),
        /// new transaction authors (author, call_id)
        NewCallAuthored(T::AccountId, [u8; 32]),
        /// add approval to transaction (multi signature account, approver)
        ApprovalAdded(T::AccountId, T::AccountId),
        /// remove approval from transaction (multi signature account, approver)
        ApprovalRemoved(T::AccountId, T::AccountId),
        /// (executor)
        CallExecuted([u8; 10]),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// duplicates in signers list
        DuplicatesInList,
        /// signer must be in signatories list
        CallerMustBeInSignatoriesList,
        /// a multi signature account with the same signatories already exists
        StorageErrorMultiSignatureAccountAlreadyExists,
        /// no multi signature account found for particular address
        StorageErrorMultiSignatureAccountNotFound,
        /// signatories must be at least 2
        SignatoriesListTooShort,
        /// signatories must be less than T::MaxSignatories
        AccountErrorSignatoriesListTooLong,
        /// min approvals must be greater than 1 and less than the number of signatories
        MinimumApprovalOutOfRange,
        /// proposed account already in the list
        AccountErrorAccountAlreadyAuthor,
        /// proposer length too long
        AccountErrorAuthorVecTooLong,
        /// error in extending authors
        AccountErrorAuthorExtendError,
        /// not part of the signatories of multi sig account so can not be proposer or sign
        AuthorNotSignatory,
        /// authors is at T::MaxSignatories - 1
        AccountErrorMaxAuthors,
        /// pending call limit defined by T::MaxPendingTransactions
        AccountErrorReachedPendingCallLimit,
        /// call is not in pending_calls
        AccountErrorCallNotFound,
        /// reached the limit of approvals set by T::MaxSignatories
        CallErrorReachedBoundedApprovalLimit,
        /// failure encoding call into `BoundedVec`. perhaps too large
        CallErrorFailureEncodingCall,
        /// user has reached the limit of multi signature accounts (T::MaxMultiSigsPerAccountId)
        AccountAtMultiSigLimit,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// create a new multi signature account
        ///
        /// signatories must have cardinality of at least 2 and
        /// less than T::MaxSignatories, with all unique elements.
        /// origin must be an element of signatories and authors a
        /// subset of signatories.
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn create_multi_sig_account(
            origin: OriginFor<T>,
            mut signatories: Vec<T::AccountId>,
            authors: Option<Vec<T::AccountId>>,
            min_approving_signatories: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            signatories.sort();
            let bounded_signatories = BoundedVec::try_from(signatories)
                .map_err(|_| Error::<T>::AccountErrorSignatoriesListTooLong)?;
            Self::validate_signatories(&bounded_signatories, &signer, min_approving_signatories)?;

            let authors = Self::prepare_authors(authors, &bounded_signatories)?;

            let msa: MultiSignatureAccount<T> =
                MultiSignatureAccount::new(bounded_signatories, authors, min_approving_signatories)
                    .map_err(Error::<T>::from)?;

            let _ = Self::add_multi_sig_account_to_storage(msa);
            //todo validate that no signatories are at their limit of MSAs.
            //todo flesh out add_multi_sig_account_to_storage
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn author_transaction(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn add_approval(origin: OriginFor<T>) -> DispatchResult {
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn remove_approval(origin: OriginFor<T>) -> DispatchResult {
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Validate the signatories list and minimum approvals
        fn validate_signatories(
            signatories: &BoundedVec<T::AccountId, T::MaxSignatories>,
            creator: &T::AccountId,
            min_approvals: u32,
        ) -> Result<(), Error<T>> {
            let signatories_len = signatories.len() as u32;
            if signatories_len < 2 {
                return Err(Error::<T>::SignatoriesListTooShort);
            }
            if !(1..=signatories_len).contains(&min_approvals) {
                return Err(Error::<T>::MinimumApprovalOutOfRange);
            }
            let mut unique = BTreeSet::new();
            let mut found_creator = false;
            for signatory in signatories.iter() {
                if !unique.insert(signatory) {
                    return Err(Error::<T>::DuplicatesInList);
                }
                if signatory == creator {
                    found_creator = true;
                }
                if let Some(user_msas) = UserMultiSigAccounts::<T>::get(signatory) {
                    if user_msas.len() as u32 >= T::MaxMultiSigsPerAccountId::get() {
                        return Err(Error::<T>::AccountAtMultiSigLimit);
                    }
                }
            }
            if !found_creator {
                return Err(Error::<T>::CallerMustBeInSignatoriesList);
            }
            Ok(())
        }
        /// prep authors for insertion into the struct
        ///
        /// signatories must be sorted
        //todo change name if signatories to sorted_signatories
        fn prepare_authors(
            authors_opt: Option<Vec<T::AccountId>>,
            signatories: &BoundedVec<T::AccountId, T::MaxSignatories>,
        ) -> Result<Option<BoundedVec<T::AccountId, T::MaxSignatories>>, Error<T>> {
            match authors_opt {
                Some(mut authors) => {
                    // If authors == signatories in size, treat it as "all are authors" → force None
                    if authors.len() == signatories.len() {
                        return Ok(None);
                    }
                    authors.sort();
                    Self::check_duplicates(&authors)?;
                    // Ensure each author is also a signatory
                    for author in &authors {
                        if signatories.as_slice().binary_search(author).is_err() {
                            return Err(Error::<T>::AuthorNotSignatory);
                        }
                    }
                    // Attempt to convert into a bounded vec
                    let authors_bounded = BoundedVec::try_from(authors)
                        .map_err(|_| Error::<T>::AccountErrorMaxAuthors)?;
                    Ok(Some(authors_bounded))
                }
                None => Ok(None),
            }
        }
        ///  check for duplicates in sorted vec
        fn check_duplicates(account_ids: &Vec<T::AccountId>) -> Result<(), Error<T>> {
            let has_duplicates = !account_ids.windows(2).all(|pair| pair[0] != pair[1]);
            if has_duplicates {
                return Err(Error::<T>::DuplicatesInList);
            }
            Ok(())
        }

        fn add_multi_sig_account_to_storage(msa: MultiSignatureAccount<T>) -> Result<(), Error<T>> {
            if MultiSignatureAccounts::<T>::contains_key(&msa.address) {
                return Err(Error::<T>::StorageErrorMultiSignatureAccountAlreadyExists.into());
            }
            //todo verify this mutate. check to see if you can explicitly define the type within the closure e.g. Option<BoundedVec<T::AccountId, T::MaxMultiSigsPerAccountId>>
            for signatory in msa.signatories.iter() {
                UserMultiSigAccounts::<T>::mutate(signatory, |user_msas_opt| {
                    if let Some(ref mut user_msas) = user_msas_opt {
                        user_msas.try_push(msa.address.clone()).unwrap();
                    } else {
                        let mut user_msas = BoundedVec::default();
                        user_msas.try_push(msa.address.clone()).unwrap();
                        *user_msas_opt = Some(user_msas);
                    }
                });
            }

            Ok(())
        }
    }
}
