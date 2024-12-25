#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
mod types;
use frame_support::BoundedVec;
pub use types::*;
pub type BoundedCallOf<T> = BoundedVec<u8, <T as Config>::MaxCallSize>;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
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
    #[pallet::storage]
    #[pallet::getter(fn multi_signature_accounts)]
    pub type MultiSignatureAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, MultiSignatureAccount<T>, OptionQuery>;

    /// this will be called by clients so they can be aware of the multi signature accounts they are signatories of. Limited to MaxMultiSigsPerAccountId
    #[pallet::storage]
    #[pallet::getter(fn user_multi_signature_accounts)]
    pub type Signatories<T: Config> = StorageMap<
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
        CallExecuted([u8; 32]),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// a multi signature account with the same signatories already exists
        //note add error that related to the creation of multi signature accounts
        MultiSignatureAccountAlreadyExists,
        MultiSignatureAccountNotFound,
        PendingTransactionAlreadyExists,
        AccountError((MultiSigAcctError)),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn author_transaction(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn add_approval(origin: OriginFor<T>) -> DispatchResult {
            Ok(())
        }

        #[pallet::call_index(2)]
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
    }
}
