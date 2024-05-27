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
        pallet,
        pallet_prelude::{DispatchResult, OptionQuery, StorageMap, ValueQuery, *},
        traits::ExistenceRequirement,
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
        // when do votes start after proposal
        type NumberOfSessionsBeforeVote: Get<u32>;
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
    #[pallet::getter(fn mining_pool_contract)]
    pub type MiningPoolContract<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

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
        ProposalFeePaid(T::AccountId, BalanceOf<T>),
        AccountLocked(T::AccountId),
        AccountUnlocked(T::AccountId),
        AccountNominatedForUnlock(T::AccountId),
        VoteStarted,
        VoteEnded(VoteResult),
        ProposalPassed,
        ProposalRejected,
    }

    #[pallet::error]
    pub enum Error<T> {
        NotValidNominator,
        ProposalFeeInsufficient,
        MiningPoolContractNotSet,
        ErrorGettingRankedNodes,
        AccountAlreadyLocked,
        ProposalAlreadyExists,
        AdminCannotBeNominated,
        LockCandidatesNotPermittedToInteract,
        LockedAccountsNotPermittedToInteract,
        LockedAccountCannotVote,
        ReferendumDoesNotExist,
        NotValidCouncilMember,
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
        pub fn set_mining_pool_contract(
            origin: OriginFor<T>,
            new_contract: T::AccountId,
        ) -> DispatchResult {
            Self::root_or_admin(origin)?;
            MiningPoolContract::<T>::put(new_contract);
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn propose_lock(
            origin: OriginFor<T>,
            account_to_lock: T::AccountId,
            proposal_fee: BalanceOf<T>,
        ) -> DispatchResult {
            let nominator = ensure_signed(origin)?;
            let _ = Self::check_action_eligibility(nominator)?;
            let _ = Self::check_nominator(nominator.clone())?;
            if let Some(admin) = PalletAdmin::<T>::get() {
                if admin == account_to_lock {
                    return Err(Error::AdminCannotBeNominated);
                }
            }
            let _ = Self::check_fee(proposal_fee)?;
            let contract_address_option = MiningPoolContract::<T>::get();
            if contract_address_option.is_none() {
                return Err(Error::MiningPoolContractNotSet);
            }
            let mining_pool = contract_address_option.unwrap();
            let _ = Self::transfer_funds(nominator.clone(), mining_pool, proposal_fee)?;
            let _ = Self::is_account_lockable(account_to_lock.clone())?;
            Self::_propose_lock(account_to_lock, nominator)?;
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn vote_on_lock(
            origin: OriginFor<T>,
            lock_candidate: T::AccountId,
            lock_decision: bool,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            let referendum_option = LockReferendums::<T>::get(lock_candidate);
            if referendum_option.is_none() {
                return Err(Error::ReferendumDoesNotExist);
            }
            let _ = Self::check_voter(voter.clone())?;
            let mut referendum = referendum_option.unwrap();
            referendum.add_vote(voter, lock_decision);
            //TODO - check if the vote has passed or failed then remove referendum and execute the action
            LockReferendums::<T>::insert(lock_candidate, referendum);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

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
            let _ = Self::check_action_eligibility(account_id)?;
            let ranked_nodes = Self::get_ranked_nodes()?;
            if let Some(index) = ranked_nodes.iter().position(|x| x == &account_id) {
                if index < T::MinNominatorRank::get() as usize {
                    return Ok(());
                }
            }
            return Err(Error::NotValidNominator);
        }

        fn check_voter(account_id: &T::AccountId) -> Result<(), Error<T>> {
            let _ = Self::check_action_eligibility(account_id)?;
            let _ = Self::check_is_council_member(account_id)?;
            Ok(())
        }

        /// is caller permitted to nominate or vote
        fn check_action_eligibility(account_id: T::AccountId) -> Result<(), Error<T>> {
            let lock_proposal_option = LockProposals::<T>::get(account_id);
            if lock_proposal_option.is_some() {
                return Err(Error::LockCandidatesNotPermittedToInteract);
            }
            let locked_account_option = LockedAccounts::<T>::get(account_id);
            if locked_account_option.is_some() {
                return Err(Error::LockedAccountsNotPermittedToInteract);
            }
            Ok(())
        }

        /// only nodes within `VotingCouncilSize` in ranked nodes can vote
        fn check_is_council_member(account_id: T::AccountId) -> Result<(), Error<T>> {
            let ranked_nodes = Self::get_ranked_nodes()?;
            if let Some(index) = ranked_nodes.iter().position(|x| x == &account_id) {
                if index < T::VotingCouncilSize::get() as usize {
                    return Ok(());
                }
            }
            return Err(Error::NotValidCouncilMember);
        }

        fn get_ranked_nodes() -> Result<Vec<T::AccountId>, Error<T>> {
            let ranked_nodes_option = T::SessionDataProvider::get_sorted_candidates();
            if ranked_nodes_option.is_none() {
                return Err(Error::ErrorGettingRankedNodes);
            }
            Ok(ranked_nodes_option.unwrap())
        }

        fn check_fee(amount_sent: BalanceOf<T>) -> Result<(), Error<T>> {
            let proposal_fee = ProposalFee::<T>::get();
            if amount_sent < proposal_fee {
                return Err(Error::ProposalFeeInsufficient);
            }
            Ok(())
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

        fn transfer_funds(
            from: T::AccountId,
            to: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)
        }

        fn _propose_lock(
            account_to_lock: T::AccountId,
            nominator: T::AccountId,
            proposal_fee: BalanceOf<T>,
        ) -> Result<(), Error<T>> {
            let proposal = LockProposal {
                proposed_account: account_to_lock.clone(),
                session_index: T::SessionDataProvider::current_session_index(),
                nominator,
            };
            LockProposals::<T>::insert(account_to_lock.clone(), proposal);
            Self::deposit_event(Event::AccountNominatedForLock(account_to_lock));
            Ok(())
        }

        fn start_pending_votes(current_session_index: SessionIndex) -> () {
            let vote_start_threshold_session =
                current_session_index - NumberOfSessionsBeforeVote::get();
            let lock_proposals = LockProposals::<T>::iter().collect::<Vec<_>>();
            for (account_id, proposal) in lock_proposals {
                if proposal.session_index <= vote_start_threshold_session {
                    let referendum = LockReferendum::<T>::new(proposal);
                    LockReferendums::<T>::insert(account_id, referendum);
                    LockProposals::<T>::remove(account_id);
                }
            }
        }

        fn end_active_votes(previous_session_index: SessionIndex) -> () {
            let lock_referendums = LockReferendums::<T>::iter().collect::<Vec<_>>();
            for (account_id, referendum) in lock_referendums {
                let assenting_votes = referendum.assenting_voters.len() as u32;
                let dissenting_votes = referendum.dissenting_voters.len() as u32;
                Self::deposit_event(Event::VoteEnded(VoteResult::Inconclusive(
                    assenting_votes,
                    dissenting_votes,
                )));
            }
        }
    }
}
