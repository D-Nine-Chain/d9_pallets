use crate::pallet::Config;
use crate::pallet::Pallet;
use crate::BalanceOf;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use sp_runtime::traits::Convert;
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
pub struct LockProposal {
    /// the account that is being voted on.
    proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    session_index: SessionIndex,
    /// who nominated this account
    nominator: T::AccountId,
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
#[scale_info(skip_type_params(T))]
/// the vote that will determine the lock status of an account
///
/// `affirmative_votes` length equal to `AssentingVotesThreshold` will lock the account
pub struct LockReferendum<T: Config> {
    nominator: T::AccountId,
    /// the account that is being voted on.
    proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    index_of_proposal: SessionIndex,
    /// accounts that voted FOR a proposal
    assenting_voters: Vec<T::AccountId>,
    /// accounts voting AGAINST a proposal
    dissenting_voters: Vec<T::AccountId>,
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
pub enum VoteResult {
    Passed,
    Rejected,
    /// (for, against)
    TimedOut(u32, u32),
}
