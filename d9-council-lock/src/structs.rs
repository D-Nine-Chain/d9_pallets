use crate::pallet::Config;
use crate::pallet::Pallet;
use crate::BalanceOf;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, BoundedVec};
use sp_runtime::traits::Convert;
use sp_staking::SessionIndex;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum ProposalType {
    AccountLock,
    ChangeDissentingThreshold,
    ChangeAssentingThreshold,
    ChangeVotingCouncilSize,
    ChangeAccountLockFee,
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
pub struct Vote<T: Config> {
    /// the account that is being voted on.
    proposed_account: T::AccountId,
    /// the index at which the proposal was made. this will determine when the vote will start.
    index_of_proposal: SessionIndex,
    /// accounts that voted FOR a proposal
    assenting_voters: Vec<T::AccountId>,
    /// accounts voting AGAINST a proposal
    dissenting_voters: Vec<T::AccountId>,
}
