use frame_support::pallet_prelude::*;
use codec::MaxEncodedLen;
#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Encode,
    Decode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen
)]
pub enum NodeTier {
    Super(SuperNodeSubTier),
    StandBy,
    Candidate,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Encode,
    Decode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen
)]
pub enum SuperNodeSubTier {
    Upper,
    Middle,
    Lower,
}
