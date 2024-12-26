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
        DispatchResult, DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo, PostDispatchInfo,
    };
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::DispatchResultWithInfo;
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

    /// key: normal accounts value: vec of multi signature accounts that user is a member of
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
        /// signer must be in signatories list
        CallerMustBeInSignatoriesList,
        /// a multi signature account with the same signatories already exists
        StorageErrorMultiSignatureAccountAlreadyExists,
        /// no multi signature account found for particular address
        StorageErrorMultiSignatureAccountNotFound,
        /// signatories must be at least 2
        AccountErrorSignatoriesListTooShort,
        /// signatories must be less than T::MaxSignatories
        AccountErrorSignatoriesListTooLong,
        /// min approvals must be greater than 1 and less than the number of signatories
        AccountErrorMinimumApprovalsRangeError, //
        /// duplicates in Authors list
        AccountErrorFoundDuplicateSigners,
        /// proposed account already in the list
        AccountErrorAccountAlreadyAuthor,
        /// proposer length too long
        AccountErrorAuthorVecTooLong,
        /// error in extending authors
        AccountErrorAuthorExtendError,
        /// not part of the signatories of multi sig account so can not be proposer or sign
        AccountErrorProposedAuthorNotSignatory,
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
            signatories: Vec<T::AccountId>,
            authors: Option<Vec<T::AccountId>>,
            min_approving_signatories: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            if !signatories.contains(&signer) {
                return Err(Error::<T>::CallerMustBeInSignatoriesList.into());
            }
            //todo validate that no signatories are at their limit of MSAs.
            //todo flesh out add_multi_sig_account_to_storage
            let bounded_signatories = BoundedVec::try_from(signatories)
                .map_err(|_| Error::<T>::AccountErrorSignatoriesListTooLong)?;
            let msa: MultiSignatureAccount<T> =
                MultiSignatureAccount::new(bounded_signatories, authors, min_approving_signatories)
                    .map_err(Error::<T>::from)?;
            let _ = Self::add_multi_sig_account_to_storage(msa);
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
        fn validate_author(account: MultiSignatureAccount<T>) {
            MultiSignatureAccounts::<T>::insert(account.address.clone(), account);
        }

        fn verify_signatory(account: MultiSignatureAccount<T>, signatory: T::AccountId) -> bool {
            account.signatories.contains(&signatory)
        }

        fn add_multi_sig_account_to_storage(account: MultiSignatureAccount<T>) {
            //todo update UserMultiSigAccounts and MultiSignatureAccounts
            MultiSignatureAccounts::<T>::insert(account.address.clone(), account);
        }
    }
}
