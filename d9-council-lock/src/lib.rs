#![cfg_attr(not(feature = "std"), no_std)]
mod structs;
pub use structs::*;
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

use frame_support::{
    traits::{Currency, LockableCurrency},
    PalletId,
};
pub use pallet::*;
use sp_staking::SessionIndex;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use codec::{Codec, MaxEncodedLen};
    use frame_support::{
        inherent::Vec,
        pallet_prelude::{DispatchResult, OptionQuery, ValueQuery, *},
        weights::Weight,
    };
    use frame_system::pallet_prelude::*;
    use pallet_d9_node_voting::NodeRewardManager;
    use sp_runtime::traits::AccountIdConversion;
    use sp_runtime::traits::{AtLeast32BitUnsigned, BadOrigin};
    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Currency: Currency<Self::AccountId>;
        type LockableCurrency: LockableCurrency<Self::AccountId>;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        // the size of the voting counil (top 27 nodes)
        type VotingCouncilSize: Get<u32>;
        // mininum account lock nominator rank
        type MinNominatorRank: Get<u32>;
        // the minimum votes to PASS account lock
        type AssentingVotesThreshold: Get<u32>;
        // minimum votes to REJECT an account block
        type DissentingVotesThreshold: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn pallet_admin)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    /// fee to proposal a lock on an account. paid by the proposer
    #[pallet::storage]
    #[pallet::getter(fn proposal_fee)]
    pub type ProposalFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AccountNominatedForLock(T::AccountId),
        AccountLocked(T::AccountId),
        AccountUnlocked(T::AccountId),
        AccountNominatedForUnlock(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        NotValidNominator,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {}

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_pallet_admin(origin: OriginFor<T>, new_admin: T::AccountId) -> DispatchResult {
            Self::root_or_admin(origin)?;
            PalletAdmin::<T>::put(new_admin);
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn propose_lock_on_account(
            origin: OriginFor<T>,
            account_to_lock: T::AccountId,
        ) -> DispatchResult {
        }
    }

    impl<T: Config> Pallet<T> {
        fn root_or_admin(origin: OriginFor<T>) -> Result<(), BadOrigin> {
            let origin = ensure_signed_or_root(origin)?;
            match origin {
                Some(caller) => {
                    let admin = PalletAdmin::<T>::get();
                    if admin.is_some() && admin.unwrap() == caller {
                        return Ok(());
                    } else {
                        return Err(BadOrigin);
                    }
                }
                None => {
                    return Ok(());
                }
            }
        }

        fn check_nominator(account_id: T::AccountId) -> Result<(), Error> {
            let ranked_nodes_option = pallet_d9_node_voting::get_sorted_candidates();
            if ranked_nodes_option.is_none() {
                return Err(Error::NotValidNominator);
            }
            let ranked_nodes = ranked_nodes_option.unwrap();
            if let Some(index) = ranked_nodes.iter().position(|x| x == &account_id) {
                if index <= MinNominatorRank as usize {
                    return Ok(());
                }
            }
            return Err(Error::NotValidNominator);
        }

        fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
    }
}
