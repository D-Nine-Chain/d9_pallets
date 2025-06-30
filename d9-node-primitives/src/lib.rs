#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode, FullCodec, MaxEncodedLen};
use frame_support::{pallet_prelude::*, BoundedVec};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

// ===== From d9-candidate-registry/src/types.rs =====
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SessionKeys<AuraId, AuthorityId, GrandpaId, ImOnlineId> {
    pub aura: AuraId,
    pub authority: AuthorityId,
    pub grandpa: GrandpaId,
    pub im_online: ImOnlineId,
}

impl<AuraId, AuthorityId, GrandpaId, ImOnlineId> MaxEncodedLen 
    for SessionKeys<AuraId, AuthorityId, GrandpaId, ImOnlineId>
where
    AuraId: MaxEncodedLen,
    AuthorityId: MaxEncodedLen,
    GrandpaId: MaxEncodedLen,
    ImOnlineId: MaxEncodedLen,
{
    fn max_encoded_len() -> usize {
        AuraId::max_encoded_len()
            .saturating_add(AuthorityId::max_encoded_len())
            .saturating_add(GrandpaId::max_encoded_len())
            .saturating_add(ImOnlineId::max_encoded_len())
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ValidationChallenge<Hash, BlockNumber> {
    pub block_hash: Hash,
    pub nonce: u64,
    pub expires_at: BlockNumber,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum ValidationFailureReason {
    InvalidSignatures,
    ChallengeExpired,
    InvalidRegistrationData,
}

// ===== From d9-candidate-registry/src/lib.rs =====
/// Trait for managing candidates
pub trait CandidateManager<AccountId, Metadata> {
    /// Add a new candidate with metadata
    fn add_candidate(who: &AccountId, metadata: Metadata) -> DispatchResult;

    /// Remove an existing candidate
    fn remove_candidate(who: &AccountId) -> DispatchResult;

    /// Check if an account is a candidate
    fn is_candidate(who: &AccountId) -> bool;
}

pub trait PromotionCriteria<AccountId> {
    type CriteriaData: FullCodec + Clone + PartialEq + Eq + TypeInfo + MaxEncodedLen + Default;
    type RegistrationData: FullCodec + Clone + PartialEq + Eq + TypeInfo + MaxEncodedLen;

    fn validate_registration_data(data: &Self::RegistrationData) -> bool;

    fn initialize_criteria_data<B>(
        who: &AccountId,
        starting_block: B,
        registration_data: &Self::RegistrationData,
    ) -> Self::CriteriaData;

    fn update_participation_data<B>(
        who: &AccountId,
        data: &mut Self::CriteriaData,
        block: B,
    ) -> DispatchResult;

    fn evaluate_promotion_eligibility(who: &AccountId, data: &Self::CriteriaData) -> bool;

    fn cleanup_criteria_data(who: &AccountId);

    fn on_participation_started(who: &AccountId, data: &Self::CriteriaData);

    fn on_participation_ended(who: &AccountId, data: &Self::CriteriaData);
}

/// Trait for querying aspirant status (implemented by registry)
pub trait AspirantRegistry<AccountId> {
    /// Check if account is an aspirant
    fn is_aspirant(who: &AccountId) -> bool;
    
    /// Get validated session keys for an aspirant
    fn get_aspirant_session_keys(who: &AccountId) -> Option<Vec<u8>>;
}

// ===== From d9-node-voting/src/types.rs =====
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub struct NodeMetadataStruct {
    pub name: BoundedVec<u8, ConstU32<128>>,
    pub sharing_percent: u8,
    pub index_of_last_percent_change: u32,
}

impl Default for NodeMetadataStruct {
    fn default() -> Self {
        Self {
            name: BoundedVec::new(),
            sharing_percent: 0,
            index_of_last_percent_change: 0,
        }
    }
}

/// Handler for candidate registry events
pub trait RegistryEventHandler<AccountId> {
    fn on_keys_validated(who: &AccountId, keys: &[u8]);
    fn on_aspirant_removed(who: &AccountId);
}

/// Handler for liveness tracking events  
pub trait LivenessEventHandler<AccountId> {
    fn on_promotion_criteria_met(who: &AccountId);
    fn on_stop_tracking(who: &AccountId);
}

/// Handler for voting system events
pub trait VotingEventHandler<AccountId> {
    fn on_candidate_added(who: &AccountId);
    fn on_candidate_removed(who: &AccountId);
}

// Provide no-op implementations for optional handlers
impl<AccountId> RegistryEventHandler<AccountId> for () {
    fn on_keys_validated(_who: &AccountId, _keys: &[u8]) {}
    fn on_aspirant_removed(_who: &AccountId) {}
}

impl<AccountId> LivenessEventHandler<AccountId> for () {
    fn on_promotion_criteria_met(_who: &AccountId) {}
    fn on_stop_tracking(_who: &AccountId) {}
}

impl<AccountId> VotingEventHandler<AccountId> for () {
    fn on_candidate_added(_who: &AccountId) {}
    fn on_candidate_removed(_who: &AccountId) {}
}