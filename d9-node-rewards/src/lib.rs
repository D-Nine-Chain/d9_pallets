#![cfg_attr(not(feature = "std"), no_std)]

use sp_staking::SessionIndex;
mod structs;
use frame_support::{traits::Currency, PalletId};
pub use pallet::*;

pub type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        inherent::Vec,
        pallet_prelude::{DispatchResult, OptionQuery, *},
        weights::Weight,
    };
    use frame_system::pallet_prelude::*;
    use pallet_d9_node_voting::NodeRewardManager;
    use sp_runtime::traits::AccountIdConversion;
    use sp_runtime::traits::BadOrigin;
    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        type CurrencySubUnits: Get<BalanceOf<Self>>;

        type Currency: Currency<Self::AccountId>;

        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // type AccountIdConversion: AccountIdConversion<Self::PalletId>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    #[pallet::storage]
    #[pallet::getter(fn node_reward_contract)]
    pub type NodeRewardContract<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pallet_admin)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ErrorIssuingRewards,
    }

    #[pallet::error]
    pub enum Error<T> {
        RestrictedAccess,
        NodeRewardContractNotSet,
        ErrorUpdatingNodeRewardContract,
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
        pub fn set_node_reward_contract(
            origin: OriginFor<T>,
            new_contract: T::AccountId,
        ) -> DispatchResult {
            Self::root_or_admin(origin)?;
            NodeRewardContract::<T>::put(new_contract);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn root_or_admin(origin: OriginFor<T>) -> Result<(), BadOrigin> {
            let caller: T::AccountId = ensure_signed(origin.clone())?;

            // Prioritize root check:
            if ensure_root(origin).is_ok() {
                return Ok(());
            }

            // Check for admin:
            if PalletAdmin::<T>::get().map_or(false, |admin| admin == caller) {
                return Ok(());
            }

            // Neither root nor admin:
            Err(BadOrigin)
        }

        fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        fn update_rewards_on_contract(
            end_index: SessionIndex,
            sorted_nodes: BoundedVec<T::AccountId, ConstU32<300>>,
        ) -> Result<(), Error<T>> {
            let sorted_nodes_vec = sorted_nodes.iter().collect::<Vec<_>>();
            //0x93440f8d
            //update_rewards
            let mut selector: Vec<u8> = [0x93, 0x44, 0x0f, 0x8d].into();
            let mut encoded_index = (end_index as u32).encode();
            let mut encoded_nodes: Vec<u8> = sorted_nodes_vec.encode();
            let mut data_for_contract_call = Vec::new();
            data_for_contract_call.append(&mut selector);
            data_for_contract_call.append(&mut encoded_index);
            data_for_contract_call.append(&mut encoded_nodes);

            let node_reward_contract_opt = NodeRewardContract::<T>::get();
            if node_reward_contract_opt.is_none() {
                return Err(Error::<T>::NodeRewardContractNotSet);
            }

            let node_reward_contract = node_reward_contract_opt.unwrap();
            let weight: Weight = Weight::from_parts(50_000_000_000, 800_000);
            let send_value: BalanceOf<T> = (0u32).into();
            let contract_call_result = pallet_contracts::Pallet::<T>::bare_call(
                Self::account_id(),
                node_reward_contract,
                send_value,
                weight,
                None,
                data_for_contract_call,
                false,
                pallet_contracts::Determinism::Enforced,
            )
            .result;
            match contract_call_result {
                Ok(_) => Ok(()),
                Err(_) => Err(Error::<T>::ErrorUpdatingNodeRewardContract),
            }
        }
    }

    impl<T: Config> NodeRewardManager<T::AccountId> for Pallet<T> {
        /// pull data to update the pool
        fn update_rewards(
            end_index: SessionIndex,
            sorted_node_list: BoundedVec<T::AccountId, ConstU32<300>>,
        ) -> () {
            let contract_update_result =
                Self::update_rewards_on_contract(end_index, sorted_node_list);
            if let Err(_) = contract_update_result {
                Self::deposit_event(Event::ErrorIssuingRewards);
                return;
            }
        }
    }
}
