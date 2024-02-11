use frame_support::pallet_prelude::*;
// use substrate_fixed::{ FixedU128, types::extra::U30 };
use codec::MaxEncodedLen;
// pub type FixedBalance = FixedU128<U30>;
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
