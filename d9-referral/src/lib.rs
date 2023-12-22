#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    // pallet_prelude imports resource such s storage, hooks dispatchResult, etc
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config {
        type RuntimeEvent: From<Event<Self, I>> +
            IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxReferralDepth: Get<u32>;
        type SetMaxReferralDepthOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
    }

    /// The current storage version.
    const STORAGE_VERSION: frame_support::traits::StorageVersion = frame_support::traits::StorageVersion::new(
        1
    );
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        NewReferralRelationshipCreated(T::AccountId, T::AccountId),
        NewReferralDepthSet(u32),
        NewDefaultParentSet(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
        NoReferralAccountRecord,
    }

    //  #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    //  pub struct ReferralAccount<AccountId> {
    //      /// the account that bequethed this account its first free balance
    //      parent: AccountId,
    //      /// all the accounts that this account bequethed
    //      children: Vec<AccountId>,
    //  }
    /// from the perspective of a node in the referral tree,
    /// the MaxReferralDepth is the largest Nth ancestor node
    /// This associated type is only necessary for first run of
    /// the chain as it will define the default value for the
    /// MaxReferralDepth storage item. This value can be changed during runtime
    /// through an extrinsic call.
    #[pallet::storage]
    #[pallet::getter(fn max_referral_depth)]
    pub type MaxReferralDepth<T: Config<I>, I: 'static = ()> = StorageValue<_, u32, ValueQuery>;

    /// (child -> parent) referral relationship AccountId -> AccountId
    #[pallet::storage]
    #[pallet::getter(fn get_parent)]
    pub type ReferralRelationships<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        T::AccountId,
        OptionQuery
    >;

    #[pallet::storage]
    pub type DirectReferralsCount<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u32,
        ValueQuery
    >;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
        /// explicitly set this value or permit the default value to persist
        pub max_referral_depth: u32,
        pub phantom: PhantomData<(T, I)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
        fn default() -> Self {
            Self {
                max_referral_depth: T::MaxReferralDepth::get(),
                phantom: PhantomData,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
        fn build(&self) {
            <MaxReferralDepth<T, I>>::set(self.max_referral_depth);
        }
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {}

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(0, 1))]
        pub fn change_referral_depth(origin: OriginFor<T>, new_depth: u32) -> DispatchResult {
            T::SetMaxReferralDepthOrigin::ensure_origin(origin)?;
            MaxReferralDepth::<T, I>::put(new_depth);
            Self::deposit_event(Event::NewReferralDepthSet(new_depth));
            Ok(())
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub fn create_referral_relationship(parent: &T::AccountId, child: &T::AccountId) {
            if parent == child {
                return;
            }

            let parent_referral_account = <ReferralRelationships<T, I>>::get(child);
            if parent_referral_account.is_some() {
                return; // child already has a parent
            }

            <ReferralRelationships<T, I>>::insert(child.clone(), parent.clone());
            let mut count = DirectReferralsCount::<T, I>::get(parent.clone());
            count += 1;
            <DirectReferralsCount<T, I>>::insert(parent.clone(), count);
            Self::deposit_event(
                Event::NewReferralRelationshipCreated(parent.clone(), child.clone())
            );
        }

        /// returns ancestors of an account
        pub fn get_ancestors(account: T::AccountId) -> Option<Vec<T::AccountId>> {
            // ensure!(
            //     ReferralRelationships::<T, I>::contains_key(account.clone()),
            //     Error::NoReferralAccountRecord
            // );
            if !ReferralRelationships::<T, I>::contains_key(account.clone()) {
                return None;
            }
            let mut ancestors: Vec<T::AccountId> = Vec::new();
            let mut current_account = account;

            for _ in 0..<MaxReferralDepth<T, I>>::get() {
                if
                    let Some(referral_account) = <ReferralRelationships<T, I>>::get(
                        &current_account
                    )
                {
                    ancestors.push(referral_account.clone());
                    current_account = referral_account;
                } else {
                    // Break if there's no referral account for the current parent
                    break;
                }
            }
            Some(ancestors)
        }

        pub fn get_direct_referral_count(account_id: T::AccountId) -> u32 {
            DirectReferralsCount::<T, I>::get(account_id)
        }
    }
}
