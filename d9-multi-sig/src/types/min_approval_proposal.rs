use crate::pallet::Config;
use frame_support::RuntimeDebugNoBound;
use frame_support::{pallet_prelude::*, BoundedVec};

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
pub struct MinimumApprovalProposal<T: Config> {
    /// address of the msa
    pub msa_address: T::AccountId,
    /// the new minimum approval threshold
    pub new_minimum: u32,
    /// address of the proposer
    pub proposer: T::AccountId,
    /// current list of approvers
    pub approvals: BoundedVec<T::AccountId, T::MaxSignatories>,
    /// Approvals needed to change min threshold
    pub pass_requirement: u32,
}
