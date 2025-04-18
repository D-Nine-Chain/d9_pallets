use crate::pallet::Config;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use sp_staking::SessionIndex;
type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;
#[derive(
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct Proposal<T: Config> {
    /// the account that is being voted on.
    pub proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    pub session_index: SessionIndex,
    /// who nominated this account
    pub nominator: T::AccountId,
    /// request to change account to this state
    pub change_to: AccountLockState,
    /// start time
    pub creation_time: MomentOf<T>,
    /// time until this proposal becomes a votea
    pub estimated_time_to_referendum: Option<MomentOf<T>>,
}

#[derive(
    PartialEqNoBound,
    EqNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
    PartialOrd,
    Ord,
    Clone,
)]
pub enum AccountLockState {
    Locked,
    Unlocked,
}

/// the vote that will determine the lock status of an account
///
/// `affirmative_votes` length equal to `AssentingVotesThreshold` will lock the account]
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct LockReferendum<T: Config> {
    pub nominator: T::AccountId,
    /// the account that is being voted on.
    pub proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    pub index_of_proposal: SessionIndex,
    /// proposed state to change account to
    pub change_to: AccountLockState,
    /// accounts that voted FOR a proposal
    pub assenting_voters: BoundedVec<T::AccountId, T::AssentingVotesThreshold>,
    /// accounts voting AGAINST a proposal
    pub dissenting_voters: BoundedVec<T::AccountId, T::DissentingVotesThreshold>,
    /// start
    pub start_time: MomentOf<T>,
    /// end
    pub estimated_end_time: MomentOf<T>,
}

impl<T: Config> LockReferendum<T> {
    pub fn new(
        proposal: Proposal<T>,
        start_time: MomentOf<T>,
        estimated_end_time: MomentOf<T>,
    ) -> Self {
        LockReferendum {
            nominator: proposal.nominator,
            proposed_account: proposal.proposed_account,
            index_of_proposal: proposal.session_index,
            assenting_voters: BoundedVec::new(),
            dissenting_voters: BoundedVec::new(),
            change_to: proposal.change_to,
            start_time,
            estimated_end_time,
        }
    }

    pub fn add_vote(
        &mut self,
        voter: T::AccountId,
        assent_on_proposal: bool,
    ) -> Result<VoteResult, T::AccountId> {
        if assent_on_proposal {
            let _ = self.assenting_voters.try_push(voter)?;
        } else {
            let _ = self.dissenting_voters.try_push(voter)?;
        }
        Ok(self.get_result())
    }

    pub fn get_result(&self) -> VoteResult {
        if self.assenting_voters.len() >= T::AssentingVotesThreshold::get() as usize {
            VoteResult::Passed
        } else if self.dissenting_voters.len() >= T::DissentingVotesThreshold::get() as usize {
            VoteResult::Rejected
        } else {
            VoteResult::Inconclusive(
                self.assenting_voters.len() as u32,
                self.dissenting_voters.len() as u32,
            )
        }
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum VoteResult {
    Passed,
    Rejected,
    /// (for, agaisnt)
    Inconclusive(u32, u32),
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub struct AccountLock<T: Config> {
    /// the account that is being locked
    pub account: T::AccountId,
    /// the account that locked the account
    pub nominator: T::AccountId,
    /// the index at which the account was locked
    pub lock_index: SessionIndex,
}

#[derive(
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct Resolution<T: Config> {
    pub nominator: T::AccountId,
    pub proposed_account: T::AccountId,
    pub index_of_proposal: SessionIndex,
    pub changed_to: AccountLockState,
    pub assenting_voters: BoundedVec<T::AccountId, T::AssentingVotesThreshold>,
    /// accounts voting AGAINST a proposal
    pub dissenting_voters: BoundedVec<T::AccountId, T::DissentingVotesThreshold>,
    pub result: VoteResult,
    pub end_time: MomentOf<T>,
}
impl<T: Config> Resolution<T> {
    pub fn new(
        lock_referendum: LockReferendum<T>,
        vote_result: VoteResult,
        end_time: MomentOf<T>,
    ) -> Self {
        Resolution {
            nominator: lock_referendum.nominator,
            proposed_account: lock_referendum.proposed_account,
            index_of_proposal: lock_referendum.index_of_proposal,
            changed_to: lock_referendum.change_to,
            assenting_voters: lock_referendum.assenting_voters,
            dissenting_voters: lock_referendum.dissenting_voters,
            result: vote_result,
            end_time,
        }
    }
}

pub trait RankingProvider<AccountId> {
    fn get_ranked_nodes() -> Option<Vec<AccountId>>;
    fn current_session_index() -> SessionIndex;
}

pub trait SessionTimeEstimator<T: Config> {
    fn est_session_total_duration() -> MomentOf<T>;
    fn est_current_session_remaining_duration() -> Option<MomentOf<T>>;
}
