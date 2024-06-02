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
pub struct VotingInterest {
    #[codec(compact)]
    pub total: u64,
    #[codec(compact)]
    pub delegated: u64,
}

impl Default for VotingInterest {
    fn default() -> Self {
        VotingInterest {
            total: 0,     // Default value for total
            delegated: 0, // Default value for delegated
        }
    }
}

impl VotingInterest {
    pub fn new(total: u64) -> Self {
        VotingInterest {
            total,
            delegated: 0,
        }
    }
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

/// defines how a user will delegate their votes among a particular candidate
#[scale_info(skip_type_params(T))]
pub struct ValidatorDelegations<T: Config> {
    pub candidate: T::AccountId,
    #[codec(compact)]
    pub votes: u64,
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
pub struct Candidate<T: Config> {
    account_id: T::AccountId,
    #[codec(compact)]
    total_votes: BalanceOf<T>,
}

pub struct ConvertAccountId<T: Config>(PhantomData<T>);
impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for ConvertAccountId<T> {
    fn convert(account_id: T::AccountId) -> Option<T::AccountId> {
        account_id.into()
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct ValidatorVoteStats<T: Config> {
    pub account_id: T::AccountId,
    pub total_votes: u64,
    pub self_votes: u64,
    pub delegated_votes: u64,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub struct NodeMetadataStruct {
    pub name: BoundedVec<u8, ConstU32<128>>,
    pub sharing_percent: u8,
    pub index_of_last_percent_change: u32,
}

pub struct ValidatorStatsOf<T: Config>(PhantomData<T>);
impl<T: Config> Convert<T::AccountId, Option<ValidatorVoteStats<T>>> for ValidatorStatsOf<T> {
    fn convert(account_id: T::AccountId) -> Option<ValidatorVoteStats<T>> {
        <Pallet<T>>::current_validator_vote_stats(account_id)
    }
}

pub trait NodeRewardManager<AccountId> {
    fn update_rewards(end_index: SessionIndex, nodes_with_votes: Vec<(AccountId, u64)>) -> ();
}

pub trait CouncilSessionManager<AccountId> {
    fn start_pending_votes(session_index: SessionIndex);
    fn end_active_votes(session_index: SessionIndex);
    fn get_ranked_nodes() -> Option<Vec<AccountId>>;
    fn current_session_index() -> SessionIndex;
}
