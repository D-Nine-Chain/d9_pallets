use crate::pallet::Config;
use crate::pallet::Pallet;
use crate::BalanceOf;
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
    assenting_voters: BoundedVec<T::AccountId, T::AssentingVotesThreshold>,
    /// accounts voting AGAINST a proposal
    dissenting_voters: BoundedVec<T::AccountId, T::DissentingVotesThreshold>,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum VoteResult {
    Passed,
    Rejected,
    /// (for, against)
    TimedOut(u32, u32),
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
