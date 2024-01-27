#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_staking::SessionIndex;
mod reward;
use reward::TierRewardPools;
pub use pallet::*;
use frame_support::traits::Currency;

pub type BalanceOf<T> =
    <<T as pallet_contracts::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use frame_system::pallet_prelude::*;
    use frame_support::{
        pallet_prelude::{ *, ValueQuery, OptionQuery, DispatchResult },
        inherent::Vec,
        BoundedVec,
        weights::Weight,
        Blake2_128Concat,
    };
    use pallet_d9_node_voting::NodeRewardManager;
    const STORAGE_VERSION: frame_support::traits::StorageVersion = frame_support::traits::StorageVersion::new(
        1
    );
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        type CurrencySubUnits: Get<BalanceOf<Self>>;

        type Currency: Currency<Self::AccountId>;

        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    ///accumulative node reward
    #[pallet::storage]
    #[pallet::getter(fn node_reward_total)]
    pub type NodeRewardTotal<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// the balance of the pool at session end
    #[pallet::storage]
    #[pallet::getter(fn session_pool_balance)]
    pub type SessionPoolBalance<T: Config> = StorageValue<_, u128, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn rewards)]
    pub type RewardBalances<T: Config> = StorageMap<_, T::AccountId, T::Balance, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn mining_pool_contract)]
    pub type MiningPoolContract<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn current_session_index)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CandidacySubmitted(T::AccountId),
        VotesDelegatedBy(T::AccountId),
        CandidacyRemoved(T::AccountId),
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        //todo something
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            //todo something
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                //todo something
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_pallet_admin(origin: OriginFor<T>, new_admin: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let current_admin = PalletAdmin::<T>::get();
            if current_admin.is_some() {
                ensure!(
                    current_admin.unwrap() == caller,
                    "Only the current admin can set a new admin"
                );
            } else {
                ensure_root(origin)?;
            }
            PalletAdmin::<T>::put(new_admin);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn update_node_rewards() -> Result<(), Error> {
            let mut tier_reward_pools = TierRewardPools::new();
            let node_reward_pool = NodeRewardTotal::<T>::get();
            tier_reward_pools.calculate_tier_reward_pools(node_reward_pool);
            Ok(())
        }

        fn get_processed_d9_total() -> Result<Balance, Error> {
            let mining_pool_contract = match MiningPoolContract::<T>::get() {
                Some(contract) => contract,
                None => {
                    return Err(Error::MiningPoolContractNotSet);
                }
            };
            let pallet_admin: <T as Config>::AccountId = match PalletAdmin::<T>::get() {
                Some(admin) => admin,
                None => {
                    return Err(Error::PalletAdminNotSet);
                }
            };

            let weight = Weight::max_value();
            let data_for_contract_call = Vec::new();
            let contract_call_result = pallet_contracts::Pallet::<T>::bare_call(
                pallet_admin,
                mining_pool_contract,
                0,
                weight,
                None,
                data_for_contract_call,
                false,
                pallet_contracts::Determinism::Enforced
            ).result;
            if let Err(e) = contract_call_result {
                return Err(e);
            }

            let total: Balance = contract_call_result.unwrap() as Balance;

            Ok(total)
        }

        fn increase_node_reward_pool(increase: BalanceOf<T>) {
            let _ = NodeRewardTotal::<T>::mutate(|total| {
                *total = total.saturating_add(increase);
            });
        }
    }

    impl<T: Config> NodeRewardManager for Pallet<T> {
        fn update_reward_pool(current_index: SessionIndex) -> () {
            let previous_index = end_index.saturating_sub(1);
            let previous_session_balance: BalanceOf<T> = match
                SessionPoolTotal::<T>::get(previous_index)
            {
                Some(balance) => balance,
                None => 0,
            };
            let ending_session_pool_balance: Balance = match Self::get_processed_d9_total() {
                Ok(balance) => balance as Balance,
                Err(_) => 0,
            };
            SessionPoolTotal::<T>::insert(end_index, ending_session_pool_balance);
            let session_delta: BalanceOf<T> =
                ending_session_pool_balance.saturating_sub(previous_session_balance);
            let three_percent = FixedBalance::from_num(3)
                .checked_div(FixedBalance::from_num(100))
                .unwrap();
            let node_reward_increase = three_percent
                .saturating_mul(session_delta)
                .to_num::<BalanceOf<T>>();
            Self::increase_node_reward_pool(node_reward_increase);
            let bounded_node_list = SessionNodeList::<T>::get(end_index);
            match bounded_node_list {
                Some(node_list) => {
                    NodeRewardManager::<T>::calculate_rewards(node_list);
                }
                None => {}
            }
        }

        fn calculate_rewards(sorted_node_list: BoundedVec<T::AccountId, ConstU32<300>>) -> () {}
    }
}
