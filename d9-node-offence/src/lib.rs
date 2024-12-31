#![cfg_attr(not(feature = "std"), no_std)]

use pallet_im_online::{IdentificationTuple, UnresponsivenessOffence};
use sp_staking::offence::{Offence, OffenceError};
use sp_staking::SessionIndex;
use sp_std::prelude::*;
mod types;
pub use pallet::*;
pub use types::*;
mod tests;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
    use frame_system::{ensure_signed, pallet_prelude::*};
    use pallet_d9_node_voting::Pallet as NodeVoting;
    use sp_staking::offence::ReportOffence;
    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_im_online::Config + pallet_d9_node_voting::Config
    {
        // Add pallet_im_online::Config
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// defines the bound of the BoundedVector for the number of offenders per session storage
        /// if this value were to change then the storage version should be updated
        type MaxOffendersPerSession: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn session_offenders)]
    pub type SessionOffenders<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        SessionIndex,
        BoundedVec<T::AccountId, T::MaxOffendersPerSession>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ValidatorDecommissioned(T::AccountId),
        ErrorDecommissioningValidator(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        SomeErrors,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn submit_candidacy(origin: OriginFor<T>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Ok(())
        }
    }

    impl<T: Config>
        ReportOffence<
            T::AccountId,
            IdentificationTuple<T>,
            UnresponsivenessOffence<IdentificationTuple<T>>,
        > for Pallet<T>
    {
        fn report_offence(
            _: Vec<T::AccountId>,
            offence: UnresponsivenessOffence<IdentificationTuple<T>>,
        ) -> Result<(), OffenceError> {
            let offenders = offence.offenders();
            for id_tuple in offenders.iter() {
                let (validator_id, _) = id_tuple;
                let encoded_validator_id = validator_id.encode();
                let account_id = T::AccountId::decode(&mut &encoded_validator_id[..]).unwrap();
                let current_session = pallet_d9_node_voting::Pallet::<T>::current_session_index();
                SessionOffenders::<T>::mutate(current_session, |offending_nodes_opt| {
                    if let Some(offending_nodes) = offending_nodes_opt.as_mut() {
                        // We rely on the bounded nature of BoundedVec to prevent overflow.
                        let _ = offending_nodes.try_push(account_id.clone());
                    } else {
                        let mut new_offending_nodes =
                            BoundedVec::<T::AccountId, T::MaxOffendersPerSession>::new();
                        let _ = new_offending_nodes.try_push(account_id.clone());
                        *offending_nodes_opt = Some(new_offending_nodes);
                    }
                });
                let decommission_result =
                    NodeVoting::<T>::decommission_candidate(account_id.clone());
                match decommission_result {
                    Ok(_) => {
                        Self::deposit_event(Event::ValidatorDecommissioned(account_id));
                    }
                    Err(_) => {
                        Self::deposit_event(Event::ErrorDecommissioningValidator(account_id));
                        return Err(OffenceError::Other(0));
                    }
                }
            }

            Ok(())
        }
        fn is_known_offence(
            _: &[IdentificationTuple<T>],
            _: &<UnresponsivenessOffence<IdentificationTuple<T>> as Offence<
                IdentificationTuple<T>,
            >>::TimeSlot, // Specify Offender type
        ) -> bool {
            false
        }
    }
}
