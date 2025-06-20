#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use sp_io::hashing::blake2_256;
    
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The origin that can modify parameters alongside root
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    /// Parameter storage - maps hash(source_id:param_id) to value
    #[pallet::storage]
    #[pallet::getter(fn parameters)]
    pub type Parameters<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        u128,
        OptionQuery
    >;

    /// Parameter constraints - maps hash(source_id:param_id) to (min, max) inclusive range
    #[pallet::storage]
    #[pallet::getter(fn parameter_constraints)]
    pub type ParameterConstraints<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        (u128, u128),
        OptionQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Parameter set successfully
        ParameterSet { 
            source_id: Vec<u8>, 
            param_id: Vec<u8>,
            old_value: Option<u128>,
            new_value: u128,
        },
        /// Parameter constraint set
        ConstraintSet {
            source_id: Vec<u8>,
            param_id: Vec<u8>,
            min_value: u128,
            max_value: u128,
        },
        /// Parameter removed
        ParameterRemoved {
            source_id: Vec<u8>,
            param_id: Vec<u8>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Value violates minimum constraint
        ValueBelowMinimum,
        /// Value violates maximum constraint
        ValueAboveMaximum,
        /// Parameter not found
        ParameterNotFound,
        /// Invalid constraint range (min > max)
        InvalidConstraintRange,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set a parameter value with constraints
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
        pub fn set_parameter(
            origin: OriginFor<T>,
            source_id: Vec<u8>,
            param_id: Vec<u8>,
            value: u128,
        ) -> DispatchResult {
            // Ensure caller is root or admin
            Self::ensure_admin_or_root(origin)?;
            
            let key = Self::generate_key(&source_id, &param_id);
            
            // Get old value for event
            let old_value = Parameters::<T>::get(key);
            
            // Check constraints if they exist
            if let Some((min, max)) = ParameterConstraints::<T>::get(key) {
                ensure!(value >= min, Error::<T>::ValueBelowMinimum);
                ensure!(value <= max, Error::<T>::ValueAboveMaximum);
            }
            
            // Store the parameter
            Parameters::<T>::insert(key, value);
            
            Self::deposit_event(Event::ParameterSet {
                source_id,
                param_id,
                old_value,
                new_value: value,
            });
            
            Ok(())
        }
        
        /// Set constraints for a parameter (admin or root only)
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_constraint(
            origin: OriginFor<T>,
            source_id: Vec<u8>,
            param_id: Vec<u8>,
            min_value: u128,
            max_value: u128,
        ) -> DispatchResult {
            // Ensure caller is root or admin
            Self::ensure_admin_or_root(origin)?;
            
            // Validate constraint range
            ensure!(min_value <= max_value, Error::<T>::InvalidConstraintRange);
            
            let key = Self::generate_key(&source_id, &param_id);
            
            // Validate existing value against new constraints if it exists
            if let Some(existing_value) = Parameters::<T>::get(key) {
                ensure!(existing_value >= min_value, Error::<T>::ValueBelowMinimum);
                ensure!(existing_value <= max_value, Error::<T>::ValueAboveMaximum);
            }
            
            ParameterConstraints::<T>::insert(key, (min_value, max_value));
            
            Self::deposit_event(Event::ConstraintSet {
                source_id,
                param_id,
                min_value,
                max_value,
            });
            
            Ok(())
        }
        
        /// Remove a parameter
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().writes(2))]
        pub fn remove_parameter(
            origin: OriginFor<T>,
            source_id: Vec<u8>,
            param_id: Vec<u8>,
        ) -> DispatchResult {
            // Ensure caller is root or admin
            Self::ensure_admin_or_root(origin)?;
            
            let key = Self::generate_key(&source_id, &param_id);
            
            Parameters::<T>::remove(key);
            ParameterConstraints::<T>::remove(key);
            
            Self::deposit_event(Event::ParameterRemoved {
                source_id,
                param_id,
            });
            
            Ok(())
        }
    }

    // Public functions for reading parameters
    impl<T: Config> Pallet<T> {
        /// Generate a key from source_id (pallet or contract) and param_id
        pub fn generate_key(source_id: &[u8], param_id: &[u8]) -> [u8; 32] {
            let mut data = Vec::new();
            data.extend_from_slice(source_id);
            data.extend_from_slice(b":");
            data.extend_from_slice(param_id);
            blake2_256(&data)
        }
        
        /// Get parameter value
        pub fn get_parameter(source_id: &[u8], param_id: &[u8]) -> Option<u128> {
            let key = Self::generate_key(source_id, param_id);
            Parameters::<T>::get(key)
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        fn ensure_admin_or_root(origin: OriginFor<T>) -> DispatchResult {
            // First try ensure_root
            if ensure_root(origin.clone()).is_ok() {
                return Ok(());
            }
            
            // Then try admin origin
            T::AdminOrigin::ensure_origin(origin)?;
            Ok(())
        }
    }
}

// Traits for other pallets to use
pub trait ParameterReader {
    fn get_parameter(source_id: &[u8], param_id: &[u8]) -> Option<u128>;
}

impl<T: Config> ParameterReader for Pallet<T> {
    fn get_parameter(source_id: &[u8], param_id: &[u8]) -> Option<u128> {
        Self::get_parameter(source_id, param_id)
    }
}