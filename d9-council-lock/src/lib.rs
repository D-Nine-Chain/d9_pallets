#![cfg_attr(not(feature = "std"), no_std)]
use pallet_timestamp::{self as timestamp};
use sp_staking::SessionIndex;
mod types;
// mod mock;
// mod tests;
use frame_support::{
    traits::{Currency, LockableCurrency, WithdrawReasons},
    PalletId,
};
pub use pallet::*;
pub use types::*;
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::{
        inherent::Vec,
        pallet_prelude::{DispatchResult, OptionQuery, StorageMap, ValueQuery, *},
        traits::ExistenceRequirement,
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::BadOrigin;
    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + scale_info::TypeInfo + timestamp::Config {
        type LockIdentifier: Get<[u8; 8]>;
        type Currency: Currency<Self::AccountId>;
        type LockableCurrency: LockableCurrency<Self::AccountId>;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        #[pallet::constant]
        type CouncilPalletId: Get<PalletId>;
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
        // prepares votes: gets valid nominators/voters turns proposal into votes
        type RankingProvider: RankingProvider<Self::AccountId>;
        type ProposalFee: Get<BalanceOf<Self>>;
    }
    type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;
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
    pub type LockDecisionProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LockDecisionProposal<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn lock_referendums)]
    pub type LockReferendums<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, LockReferendum<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn concluded_lock_referendums)]
    pub type ConcludedLockReferendums<T: Config> = StorageNMap<
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
        ///voter, proposed account lock state, decision
        VoteRecorded(T::AccountId, T::AccountId, AccountLockState, bool),
        AccountLocked(T::AccountId),
        AccountUnlocked(T::AccountId),
        AccountNominatedForUnlock(T::AccountId),
        VoteStarted,
        VoteEnded(T::AccountId, VoteResult),
    }

    #[pallet::error]
    pub enum Error<T> {
        NotValidNominator,
        ProposalFeeInsufficient,
        MiningPoolContractNotSet,
        ErrorGettingRankedNodes,
        AccountAlreadyLocked,
        AccountNotLocked,
        ProposalAlreadyExists,
        AdminCannotBeNominated,
        LockCandidatesNotPermittedToInteract,
        LockedAccountsNotPermittedToInteract,
        LockedAccountCannotVote,
        ReferendumDoesNotExist,
        NotValidCouncilMember,
        ErrorCalculatingVotes,
        VoterAlreadyVoted,
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
            let now = Self::get_time_stamp();
            let lock_proposal = LockDecisionProposal {
                proposed_account: account_to_lock.clone(),
                session_index: T::RankingProvider::current_session_index(),
                nominator: nominator.clone(),
                change_to: AccountLockState::Locked,
                start_time: now,
                end_time: None,
            };
            Self::process_lock_decision_proposal(lock_proposal, proposal_fee)
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn propose_unlock(
            origin: OriginFor<T>,
            account_to_unlock: T::AccountId,
            proposal_fee: BalanceOf<T>,
        ) -> DispatchResult {
            let nominator = ensure_signed(origin)?;
            let now = Self::get_time_stamp();
            let unlock_proposal = LockDecisionProposal {
                proposed_account: account_to_unlock.clone(),
                session_index: T::RankingProvider::current_session_index(),
                nominator: nominator.clone(),
                change_to: AccountLockState::Unlocked,
                start_time: now,
                end_time: None,
            };
            Self::process_lock_decision_proposal(unlock_proposal, proposal_fee)
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn vote_in_referendum(
            origin: OriginFor<T>,
            lock_candidate: T::AccountId,
            assent_on_decision: bool,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            let referendum_option = LockReferendums::<T>::get(lock_candidate.clone());
            if referendum_option.is_none() {
                return Err(Error::<T>::ReferendumDoesNotExist.into());
            }
            let referendum = referendum_option.unwrap();
            let _ = Self::check_voter(&voter, &referendum)?;
            let result = Self::process_vote(voter, assent_on_decision, referendum);

            match result {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_proposal_fee(origin: OriginFor<T>, new_fee: BalanceOf<T>) -> DispatchResult {
            Self::root_or_admin(origin)?;
            ProposalFee::<T>::put(new_fee);
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

        fn process_lock_decision_proposal(
            lock_decision_proposal: LockDecisionProposal<T>,
            proposal_fee: BalanceOf<T>,
        ) -> DispatchResult {
            let _ = Self::check_lock_decision_proposal(&lock_decision_proposal)?;
            let _ = Self::check_fee(proposal_fee)?;
            let mining_pool_option = MiningPoolContract::<T>::get();
            if mining_pool_option.is_none() {
                return Err(Error::<T>::MiningPoolContractNotSet.into());
            }
            let mining_pool = mining_pool_option.unwrap();
            let _ = Self::transfer_funds(
                lock_decision_proposal.nominator.clone(),
                mining_pool,
                proposal_fee,
            )?;
            Self::save_proposal(lock_decision_proposal);
            Ok(())
        }

        fn check_lock_decision_proposal(
            lock_proposal: &LockDecisionProposal<T>,
        ) -> Result<(), Error<T>> {
            let _ = Self::check_nominator(&lock_proposal.nominator)?;
            if let Some(admin) = PalletAdmin::<T>::get() {
                if admin == lock_proposal.proposed_account {
                    return Err(Error::<T>::AdminCannotBeNominated.into());
                }
            }
            match lock_proposal.change_to {
                AccountLockState::Locked => {
                    let _ = Self::is_account_lockable(&lock_proposal.proposed_account)?;
                }
                AccountLockState::Unlocked => {
                    let _ = Self::is_account_unlockable(&lock_proposal.proposed_account)?;
                }
            }
            Ok(())
        }
        fn get_time_stamp() -> MomentOf<T> {
            timestamp::Pallet::<T>::get()
        }
        //// validate origin is permitted to nominate
        fn check_nominator(account_id: &T::AccountId) -> Result<(), Error<T>> {
            let _ = Self::check_action_eligibility(account_id)?;
            let ranked_nodes = Self::get_ranked_nodes()?;
            if let Some(index) = ranked_nodes.iter().position(|x| x == account_id) {
                if index < T::MinNominatorRank::get() as usize {
                    return Ok(());
                }
            }
            return Err(Error::<T>::NotValidNominator);
        }

        fn check_voter(
            account_id: &T::AccountId,
            referendum: &LockReferendum<T>,
        ) -> Result<(), Error<T>> {
            let _ = Self::check_action_eligibility(account_id)?;
            let _ = Self::check_is_council_member(account_id)?;
            let _ = Self::check_only_single_vote(account_id, referendum)?;
            Ok(())
        }

        /// is caller permitted to nominate or vote
        fn check_action_eligibility(account_id: &T::AccountId) -> Result<(), Error<T>> {
            let locked_account_option = LockedAccounts::<T>::get(account_id);
            if locked_account_option.is_some() {
                return Err(Error::<T>::LockedAccountsNotPermittedToInteract);
            }
            Ok(())
        }

        /// only nodes within `VotingCouncilSize` in ranked nodes can vote
        fn check_is_council_member(account_id: &T::AccountId) -> Result<(), Error<T>> {
            let ranked_nodes = Self::get_ranked_nodes()?;
            if let Some(index) = ranked_nodes.iter().position(|x| x == account_id) {
                if index < T::VotingCouncilSize::get() as usize {
                    return Ok(());
                }
            }
            return Err(Error::<T>::NotValidCouncilMember);
        }

        // only single vote per referendum per voter
        fn check_only_single_vote(
            account_id: &T::AccountId,
            referendum: &LockReferendum<T>,
        ) -> Result<(), Error<T>> {
            if referendum.assenting_voters.contains(&account_id)
                || referendum.dissenting_voters.contains(&account_id)
            {
                return Err(Error::<T>::VoterAlreadyVoted);
            }
            Ok(())
        }

        fn check_fee(amount_sent: BalanceOf<T>) -> Result<(), Error<T>> {
            let proposal_fee = ProposalFee::<T>::get();
            if amount_sent < proposal_fee {
                return Err(Error::<T>::ProposalFeeInsufficient);
            }
            Ok(())
        }

        fn save_proposal(lock_proposal: LockDecisionProposal<T>) -> () {
            Self::deposit_event(Event::AccountNominatedForLock(
                lock_proposal.proposed_account.clone(),
            ));
            LockDecisionProposals::<T>::insert(
                lock_proposal.proposed_account.clone(),
                lock_proposal,
            );
        }
        fn get_ranked_nodes() -> Result<Vec<T::AccountId>, Error<T>> {
            let ranked_nodes_option = T::RankingProvider::get_ranked_nodes();
            if ranked_nodes_option.is_none() {
                return Err(Error::<T>::ErrorGettingRankedNodes);
            }
            Ok(ranked_nodes_option.unwrap())
        }

        fn process_vote(
            voter_id: T::AccountId,
            decision: bool,
            mut referendum: LockReferendum<T>,
        ) -> Result<VoteResult, Error<T>> {
            let add_vote_result = referendum.add_vote(voter_id.clone(), decision);
            if add_vote_result.is_err() {
                return Err(Error::ErrorCalculatingVotes);
            }
            let vote_result = add_vote_result.unwrap();
            Self::deposit_event(Event::VoteRecorded(
                voter_id.clone(),
                referendum.proposed_account.clone(),
                referendum.change_to.clone(),
                decision,
            ));
            let proposed_account = referendum.proposed_account.clone();
            if vote_result == VoteResult::Passed || vote_result == VoteResult::Rejected {
                Self::deposit_event(Event::VoteEnded(
                    proposed_account.clone(),
                    vote_result.clone(),
                ));
                Self::execute_referendum(&referendum)?;
                referendum.end_time = Some(Self::get_time_stamp());
                ConcludedLockReferendums::<T>::insert(
                    (
                        T::RankingProvider::current_session_index(),
                        proposed_account.clone(),
                    ),
                    referendum,
                );
                LockReferendums::<T>::remove(proposed_account);
            } else {
                LockReferendums::<T>::insert(proposed_account, referendum);
            }

            Ok(vote_result)
        }

        /// lock or unlock account funds based on referendum result
        fn execute_referendum(referendum: &LockReferendum<T>) -> Result<(), Error<T>> {
            match referendum.change_to {
                AccountLockState::Locked => {
                    LockedAccounts::<T>::insert(
                        referendum.proposed_account.clone(),
                        AccountLock {
                            account: referendum.proposed_account.clone(),
                            nominator: referendum.nominator.clone(),
                            lock_index: T::RankingProvider::current_session_index(),
                        },
                    );
                    Self::lock_funds(&referendum.proposed_account);
                    Self::deposit_event(Event::AccountLocked(referendum.proposed_account.clone()));
                }
                AccountLockState::Unlocked => {
                    LockedAccounts::<T>::remove(referendum.proposed_account.clone());
                    Self::unlock_funds(&referendum.proposed_account);
                    Self::deposit_event(Event::AccountUnlocked(
                        referendum.proposed_account.clone(),
                    ));
                }
            }
            Ok(())
        }

        fn is_account_lockable(account_id: &T::AccountId) -> Result<(), Error<T>> {
            let existing_proposal = LockDecisionProposals::<T>::get(account_id.clone());
            if existing_proposal.is_some() {
                return Err(Error::<T>::ProposalAlreadyExists);
            }
            let locked_account = LockedAccounts::<T>::get(account_id);
            if locked_account.is_some() {
                return Err(Error::<T>::AccountAlreadyLocked);
            }
            return Ok(());
        }

        fn is_account_unlockable(account_id: &T::AccountId) -> Result<(), Error<T>> {
            let existing_proposal = LockDecisionProposals::<T>::get(account_id.clone());
            if existing_proposal.is_some() {
                return Err(Error::<T>::ProposalAlreadyExists);
            }
            let locked_account = LockedAccounts::<T>::get(account_id);
            if locked_account.is_none() {
                return Err(Error::<T>::AccountNotLocked);
            }
            return Ok(());
        }

        /// lock user funds
        fn lock_funds(account_id: &T::AccountId) -> () {
            T::LockableCurrency::set_lock(
                T::LockIdentifier::get(),
                account_id,
                T::LockableCurrency::total_issuance(),
                WithdrawReasons::all(),
            );
        }

        /// unlock user funds
        fn unlock_funds(account_id: &T::AccountId) -> () {
            T::LockableCurrency::remove_lock(T::LockIdentifier::get(), &account_id);
        }

        fn transfer_funds(
            from: T::AccountId,
            to: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)
        }

        pub fn start_pending_votes(current_session_index: SessionIndex) -> () {
            let vote_start_threshold_session =
                current_session_index - T::NumberOfSessionsBeforeVote::get();
            let lock_proposals = LockDecisionProposals::<T>::iter().collect::<Vec<_>>();
            for (account_id, proposal) in lock_proposals {
                if proposal.session_index <= vote_start_threshold_session {
                    let referendum = LockReferendum::<T>::new(proposal, Self::get_time_stamp());
                    LockReferendums::<T>::insert(account_id.clone(), referendum);
                    LockDecisionProposals::<T>::remove(account_id);
                }
            }
        }

        pub fn end_active_votes(ending_index: SessionIndex) -> () {
            let lock_referendums = LockReferendums::<T>::iter().collect::<Vec<_>>();
            let now = Self::get_time_stamp();
            for (account_id, mut referendum) in lock_referendums {
                Self::deposit_event(Event::VoteEnded(
                    account_id.clone(),
                    VoteResult::Inconclusive(
                        referendum.assenting_voters.len() as u32,
                        referendum.dissenting_voters.len() as u32,
                    ),
                ));
                referendum.end_time = Some(now);
                ConcludedLockReferendums::<T>::insert(
                    (ending_index, account_id.clone()),
                    referendum,
                );
                LockReferendums::<T>::remove(account_id);
            }
        }
    }
}
