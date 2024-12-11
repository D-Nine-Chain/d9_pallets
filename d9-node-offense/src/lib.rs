#![cfg_attr(not(feature = "std"), no_std)]
use sp_staking::offence::{ Kind, Offence, ReportOffence};
use pallet_session::{IdentificationTuple, SessionIndex, ValidatorId};
use pallet_im_online::offence::UnresponsivenessOffence;
use pallet_session::ValidatorSetWithIdentification;

mod types;
pub use pallet::*;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{inherent::Vec, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use sp_staking::offence::ReportOffence;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type
    }

    #[pallet::storage]
    #[pallet::getter(fn node_votes)]
    pub type NodeAccumulativeVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {}
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {}
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn submit_candidacy() -> DispatchResult {
            Ok(())
        }
    }
    impl<T: Config> Pallet<T> {}

    impl<T: Config> ReportOffence<
        T::AccountId,
        IdentificationTuple<T>,
        UnresponsivenessOffence<IdentificationTuple<T>>
    > for Pallet<T> {
        fn report_offence(
            reporters: Vec<T::AccountId>,
            offence: UnresponsivenessOffence<IdentificationTuple<T>>
        ) -> Result<(), OffenceError> {
            // Your implementation logic
            Ok(())
        }

        fn is_known_offence(
            offenders: &[IdentificationTuple<T>],
            time_slot: &<UnresponsivenessOffence<IdentificationTuple<T>>> as Offence<_>>::TimeSlot
        ) -> bool {
            // Your check logic
            false
        }
    }

}
