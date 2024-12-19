use crate::pallet::Config;
use crate::pallet::Pallet;
use codec::MaxEncodedLen;
use frame_support::pallet_prelude::*;
use frame_support::RuntimeDebugNoBound;

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
