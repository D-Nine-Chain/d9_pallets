use crate::pallet::Config;
use crate::pallet::Pallet;
use codec::MaxEncodedLen;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*, traits::ValidatorSet};
use pallet_im_online::ValidatorId;
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
