#![cfg_attr(not(feature = "std"), no_std)]
use sp_staking::SessionIndex;
use sp_std::prelude::*;
mod structs;
use frame_support::{
    traits::{Currency, LockableCurrency},
    PalletId,
};
pub use pallet::*;

pub use structs::*;
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use codec::{Codec, MaxEncodedLen};
    use frame_support::{
        inherent::Vec,
        pallet_prelude::{DispatchResult, OptionQuery, StorageMap, ValueQuery, *},
        weights::Weight,
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;

    use sp_runtime::traits::AccountIdConversion;
    use sp_runtime::traits::{AtLeast32BitUnsigned, BadOrigin};
    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + scale_info::TypeInfo {
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
        // something to get index and candidate list
        type SessionDataProvider: D9SessionDataProvider<Self::AccountId>;
    }
    //NOTE - if these values are to be changed then let it be done by a seperate pallet that will hav

    #[pallet::storage]
    #[pallet::getter(fn pallet_admin)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    /// fee to proposal a lock on an account. paid by the proposer
    #[pallet::storage]
    #[pallet::getter(fn proposal_fee)]
    pub type ProposalFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn lock_proposals)]
    pub type LockProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LockProposal<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_lock_referendums)]
    pub type LockReferendums<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LockReferendum<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn concluded_lock_referendums)]
    pub type ConcludedLockVotes<T: Config> = StorageNMap<
        Key = (
            NMapKey<Blake2_128Concat, SessionIndex>,
            NMapKey<Blake2_128Concat, T::AccountId>,
        ),
        Value = LockReferendum<T>,
        QueryKind = OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn locked_accounts)]
    pub type LockedAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountLock<T>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AccountNominatedForLock(T::AccountId),
        AccountLocked(T::AccountId),
        AccountUnlocked(T::AccountId),
        AccountNominatedForUnlock(T::AccountId),
        VoteStarted,
        VoteEnded,
        ProposalPassed,
        ProposalRejected,
    }

    #[pallet::error]
    pub enum Error<T> {
        NotValidNominator,
        ErrorGettingRankedNodes,
        AccountAlreadyLocked,
        ProposalAlreadyExists,
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
            let nominator = ensure_signed(origin)?;
            let _ = Self::check_nominator(nominator.clone())?;
            let _ = Self::is_account_lockable(account_to_lock.clone())?;
            Ok(())
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

        /// validate origin is permitted to nominate
        fn check_nominator(account_id: T::AccountId) -> Result<(), Error<T>> {
            let ranked_nodes_option = T::SessionDataProvider::get_sorted_candidates();
            if ranked_nodes_option.is_none() {
                return Err(Error::ErrorGettingRankedNodes);
            }
            let ranked_nodes = ranked_nodes_option.unwrap();
            if let Some(index) = ranked_nodes.iter().position(|x| x == &account_id) {
                if index < T::MinNominatorRank::get() as usize {
                    return Ok(());
                }
            }
            return Err(Error::NotValidNominator);
        }

        fn is_account_lockable(account_id: T::AccountId) -> Result<(), Error<T>> {
            let existing_proposal = LockProposals::<T>::get(account_id.clone());
            if existing_proposal.is_some() {
                return Err(Error::ProposalAlreadyExists);
            }
            let locked_account = LockedAccounts::<T>::get(account_id);
            if locked_account.is_some() {
                return Err(Error::AccountAlreadyLocked);
            }

            return Ok(());
        }

        fn _propose_lock(
            account_id: T::AccountId,
            nominator: T::AccountId,
        ) -> Result<(), Error<T>> {
            let proposal = LockProposal {
                proposed_account: account_id.clone(),
                session_index: T::SessionDataProvider::current_session_index(),
                nominator,
            };
            LockProposals::<T>::insert(account_id.clone(), proposal);
            Self::deposit_event(Event::AccountNominatedForLock(account_id));
            Ok(())
        }
        fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
    }
}
