use crate::pallet::Config;
use crate::pallet::Error;
use crate::BoundedCallOf;
use codec::MaxEncodedLen;
use frame_support::storage::bounded_btree_set::BoundedBTreeSet;
use frame_support::storage::bounded_vec::BoundedVec;
use frame_support::RuntimeDebugNoBound;
use frame_support::{inherent::Vec, pallet_prelude::*};
use sp_io::hashing::blake2_256;
use sp_std::boxed::Box;
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
pub struct PendingCall<T: Config> {
    pub id: [u8; 32],
    pub call: BoundedCallOf<T>,
    pub approvals: BoundedBTreeSet<T::AccountId, T::MaxSignatories>,
}

impl<T: Config> PendingCall<T> {
    pub fn new(
        call: Box<<T as Config>::RuntimeCall>,
        author: T::AccountId,
    ) -> Result<Self, PendingCallError> {
        let mut approvals = BoundedBTreeSet::new();
        let _ = approvals.try_insert(author);
        Self::encode_call(call).map(|call| {
            let id = Self::generate_call_id(&call);
            Self {
                id,
                call,
                approvals,
            }
        })
    }

    pub fn add_approval(&mut self, signatory: T::AccountId) -> Result<u32, PendingCallError> {
        if self.approvals.try_insert(signatory).is_err() {
            return Err(PendingCallError::ReachedBoundedApprovalLimit);
        }
        Ok(self.approvals.len() as u32)
    }

    pub fn remove_approval(&mut self, author: T::AccountId) -> Result<(), PendingCallError> {
        let _ = self.approvals.remove(&author);
        Ok(())
    }

    fn encode_call(
        call: Box<<T as Config>::RuntimeCall>,
    ) -> Result<BoundedCallOf<T>, PendingCallError> {
        let mut encoded = Vec::new();
        call.encode_to(&mut encoded);

        let result = BoundedVec::try_from(encoded);
        result.map_err(|_| PendingCallError::FailureEncodingCall)
    }

    pub fn decode_call(&self) -> Result<Box<<T as Config>::RuntimeCall>, PendingCallError> {
        let call = <T as Config>::RuntimeCall::decode(&mut &self.call[..])
            .map_err(|_| PendingCallError::FailureDecodingCall)?;

        Ok(Box::new(call))
    }

    /// Generate a unique, deterministic ID for the given call.
    ///
    /// The `call` bytes are combined with a domain-separation tag (`"d9-call-id"`)
    fn generate_call_id(call: &BoundedCallOf<T>) -> [u8; 32] {
        let current_timestamp = <pallet_timestamp::Pallet<T>>::get().encode();
        let preimage = (b"d9-call-id", call, current_timestamp).encode();
        blake2_256(&preimage)
    }
}

#[derive(
    PartialEqNoBound, EqNoBound, CloneNoBound, Encode, Decode, RuntimeDebugNoBound, TypeInfo,
)]
#[scale_info(skip_type_params(T))]
pub enum PendingCallError {
    ReachedBoundedApprovalLimit,
    FailureEncodingCall,
    FailureDecodingCall,
}
impl<T> From<PendingCallError> for Error<T> {
    fn from(err: PendingCallError) -> Self {
        match err {
            PendingCallError::ReachedBoundedApprovalLimit => Error::<T>::ApprovalsLimitReached,
            PendingCallError::FailureEncodingCall => Error::<T>::CallEncodingFailure,
            PendingCallError::FailureDecodingCall => Error::<T>::FailureDecodingCall,
        }
    }
}
