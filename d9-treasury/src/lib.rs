#![cfg_attr(not(feature = "std"), no_std)]

// #[cfg(test)]
// mod tests;
// pub mod weights;

// use pallet_treasury::Config as TreasuryConfig;
// pub use weights::WeightInfo;
pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{ *, OptionQuery },
		traits::{ EnsureOrigin, OnUnbalanced, Currency, Imbalance },
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{ traits::AtLeast32BitUnsigned, FixedPointOperand };
	use sp_std::fmt::Debug;
	use codec::Codec;
	pub type PositiveImbalanceOf<
		T,
		I = ()
	> = <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::PositiveImbalance;

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);
	//73chn1c41/5h1b80137h
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		type Balance: Parameter +
			Member +
			AtLeast32BitUnsigned +
			Codec +
			Default +
			Copy +
			MaybeSerializeDeserialize +
			Debug +
			MaxEncodedLen +
			TypeInfo +
			FixedPointOperand;
		type RuntimeEvent: From<Event<Self, I>> +
			IsType<<Self as frame_system::Config>::RuntimeEvent>;
		// type EnsureTreasurer: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
		type Currency: Currency<Self::AccountId>;
		type MaxSpendPerTransaction: Get<Self::Balance>;
		// type OnUnbalance: OnUnbalanced<<Self::Currency as Currency<Self::AccountId>>::PositiveImbalance>;
	}

	/// Number of proposals that have been made.
	#[pallet::storage]
	#[pallet::getter(fn treasurer)]
	pub type Treasurer<T: Config<I>, I: 'static = ()> = StorageValue<
		_,
		Option<T::AccountId>,
		OptionQuery
	>;

	#[derive(frame_support::DefaultNoBound)]
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub treasurer: Option<T::AccountId>,
		pub _marker: PhantomData<I>,
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			if let Some(ref treasurer_account) = self.treasurer {
				Treasurer::<T, I>::put(Some(treasurer_account));
			}
		}
	}

	// impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
	// 	fn default() -> Self {
	// 		// Default values for your genesis config
	// 		Self {
	// 			treasurer: Default::default(),
	// 			_marker: Default::default(),
	// 		}
	// 	}
	// }

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		NewTreasurer(T::AccountId),
	}

	/// Error for the treasury pallet.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		OnlyTreasurerCanDoThis,
		NoTreasurerSet,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::call_index(0)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(1, 1) // Reading current treasurer and writing new treasurer
			// (10_000_u64).into() // Some arbitrary computation weight
		)]
		pub fn new_treasurer(origin: OriginFor<T>, new_treasurer: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let current_treasurer = Self::treasurer()
				.ok_or(Error::<T, I>::NoTreasurerSet)?
				.unwrap();
			ensure!(caller == current_treasurer, Error::<T, I>::OnlyTreasurerCanDoThis);
			Self::deposit_event(Event::NewTreasurer(new_treasurer));
			Ok(())
		}
	}
	pub struct EnsureTreasurerLimit<T: Config<I>, I: 'static>(sp_std::marker::PhantomData<(T, I)>);

	impl<T: Config<I>, I> EnsureOrigin<T::RuntimeOrigin> for EnsureTreasurerLimit<T, I> {
		type Success = T::Balance;

		fn try_origin(o: OriginFor<T>) -> Result<Self::Success, OriginFor<T>> {
			let caller = match ensure_signed(o.clone()) {
				Ok(caller) => caller,
				Err(_) => {
					return Err(o.clone());
				}
			};
			let current_treasurer = Treasurer::<T, I>::get().unwrap();

			if let Some(current_treasurer) = current_treasurer {
				if caller == current_treasurer {
					Ok(T::MaxSpendPerTransaction::get())
				} else {
					Err(o)
				}
			} else {
				Err(o)
			}
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<T::RuntimeOrigin, ()> {
			use frame_system::RawOrigin;
			let treasurer = Treasurer::<T, I>::get()
				.ok_or(())?
				.ok_or(())?;
			Ok(T::RuntimeOrigin::from(RawOrigin::Signed(treasurer)))
		}
	}
	/// Provides a way to ensure that only the treasurer can execute certain actions.
	///
	/// This struct acts as a guard to check that the origin of a call is indeed the treasurer.
	/// It leverages the Substrate's `EnsureOrigin` trait to perform the check.
	///
	/// # Usage
	///
	/// It can be used in runtime modules where certain operations are restricted to the treasurer.
	/// The treasurer's account ID is fetched from storage for verification.
	///
	/// # Type Parameters
	///
	/// - `T`: Represents the runtime configuration.
	/// - `I`: A lifetime parameter.
	///
	/// # Example
	///
	/// If used as an origin in a dispatchable call, it will ensure that the call will only be
	/// successful if it is initiated by the treasurer.
	///
	pub struct EnsureTreasurer<T: Config<I>, I: 'static>(sp_std::marker::PhantomData<(T, I)>);
	impl<T: Config<I>, I> EnsureOrigin<T::RuntimeOrigin> for EnsureTreasurer<T, I> {
		type Success = T::AccountId;

		fn try_origin(o: OriginFor<T>) -> Result<Self::Success, OriginFor<T>> {
			let caller = match ensure_signed(o.clone()) {
				Ok(caller) => caller,
				Err(_) => {
					return Err(o.clone());
				}
			};
			let current_treasurer = Treasurer::<T, I>::get().unwrap();

			if let Some(current_treasurer) = current_treasurer {
				if caller == current_treasurer { Ok(caller) } else { Err(o) }
			} else {
				Err(o)
			}
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<T::RuntimeOrigin, ()> {
			use frame_system::RawOrigin;
			let treasurer = Treasurer::<T, I>::get()
				.ok_or(())?
				.ok_or(())?;
			Ok(T::RuntimeOrigin::from(RawOrigin::Signed(treasurer)))
		}
	}

	pub struct RewardBalancer<T: Config<I>, I: 'static>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> OnUnbalanced<PositiveImbalanceOf<T, I>>
	for RewardBalancer<T, I> {
		fn on_nonzero_unbalanced(amount: PositiveImbalanceOf<T, I>) {
			let numeric_amount = amount.peek();
			let _ = T::Currency::burn(numeric_amount);
		}
	}
}
