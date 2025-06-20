use codec::{Encode, Decode};
use frame_support::pallet_prelude::*;
use sp_runtime::RuntimeDebug;

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