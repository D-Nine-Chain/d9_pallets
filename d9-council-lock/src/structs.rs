use crate::pallet::Config;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use sp_staking::SessionIndex;

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
pub struct LockProposal<T: Config> {
    /// the account that is being voted on.
    pub proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    pub session_index: SessionIndex,
    /// who nominated this account
    pub nominator: T::AccountId,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
/// the vote that will determine the lock status of an account
///
/// `affirmative_votes` length equal to `AssentingVotesThreshold` will lock the account]
#[scale_info(skip_type_params(T))]

pub struct LockReferendum<T: Config> {
    nominator: T::AccountId,
    /// the account that is being voted on.
    proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    index_of_proposal: SessionIndex,
    /// accounts that voted FOR a proposal
    pub assenting_voters: BoundedVec<T::AccountId, T::AssentingVotesThreshold>,
    /// accounts voting AGAINST a proposal
    pub dissenting_voters: BoundedVec<T::AccountId, T::DissentingVotesThreshold>,
}

impl<T: Config> LockReferendum<T> {
    pub fn new(proposal: LockProposal<T>) -> Self {
        LockReferendum {
            nominator: proposal.nominator,
            proposed_account: proposal.proposed_account,
            index_of_proposal: proposal.session_index,
            assenting_voters: BoundedVec::new(),
            dissenting_voters: BoundedVec::new(),
        }
    }

    pub fn add_vote(&mut self, voter: T::AccountId, decision: bool) -> VoteResult {
        if decision {
            let _ = self.assenting_voters.try_push(voter);
        } else {
            let _ = self.dissenting_voters.try_push(voter);
        }
        self.get_result()
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
    account: T::AccountId,
    /// the account that locked the account
    nominator: T::AccountId,
    /// the index at which the account was locked
    lock_index: SessionIndex,
}

pub trait D9SessionDataProvider<AccountId> {
    fn get_sorted_candidates() -> Option<Vec<AccountId>>;
    fn current_session_index() -> SessionIndex;
}
