#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::StorageVersion;
    use frame_system::pallet_prelude::*;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxSignatories: Get<u32>;
        type MaxTransactionSize: Get<u32>;
        type MaxMultiSigsPerAccountId: Get<u32>;
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
        /// (executor)
        MultiSignatureTransactionExecuted(T::AccountId),
        // note transaction executed and failure events need to be fleshed out
        /// failure
        MultiSignatureTransactionFailed(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// a multi signature account with the same signatories already exists
        //note add error that related to the creation of multi signature accounts
        MultiSignatureAccountAlreadyExists,
        MultiSignatureAccountNotFound,
        PendingTransactionAlreadyExists,
    }
    impl<T: Config> Pallet<T> {
        fn save_account(account: MultiSignatureAccount<T>) {
            MultiSignatureAccounts::<T>::insert(account.address.clone(), account);
        }
    }
}
