#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{inherent::Vec, pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use pallet_d9_candidate_registry::PromotionCriteria;
use sp_application_crypto::AppCrypto;
use sp_runtime::{
    offchain::storage_lock::{BlockAndTime, StorageLock},
    traits::{BadOrigin, Saturating},
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        ValidTransaction,
    },
};
use sp_std::vec;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_session::Config + pallet_d9_candidate_registry::Config
    where
        Self: TypeInfo,
    {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum proofs stored per participant before rotation
        #[pallet::constant]
        type MaxProofLength: Get<u32>;

        /// Minimum proofs required in a window to qualify
        #[pallet::constant]
        type MinWindowThreshold: Get<u32>;

        /// Minimum completed windows to become candidate
        #[pallet::constant]
        type MinQualifyingWindows: Get<u32>;

        /// Blocks per qualification window
        #[pallet::constant]
        type WindowSizeBlocks: Get<BlockNumberFor<Self>>;

        /// Authority ID for signing proofs
        type ProofAuthorityId: AppCrypto + Member + Parameter;

        /// The identifier type for an offchain worker.
        type AuthorityId: AppCrypto + Member + Parameter;
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct LivenessProof<T: Config> {
        pub block_hash: T::Hash,
        pub block_number: BlockNumberFor<T>,
        pub parent_hash: T::Hash,
        pub state_root: T::Hash,
        pub extrinsics_root: T::Hash,
        pub extrinsic_count: u32,
        pub previous_signature: Option<BoundedVec<u8, ConstU32<64>>>,
        pub signature: BoundedVec<u8, ConstU32<64>>,
    }

    impl<T: Config> sp_std::fmt::Debug for LivenessProof<T> {
        fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
            f.debug_struct("LivenessProof")
                .field("block_number", &self.block_number)
                .field("extrinsic_count", &self.extrinsic_count)
                .finish()
        }
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct HeaderData<T: Config> {
        pub block_number: BlockNumberFor<T>,
        pub block_hash: T::Hash,
        pub parent_hash: T::Hash,
        pub state_root: T::Hash,
        pub extrinsics_root: T::Hash,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct LivenessTracker<T: Config> {
        pub completed_windows: u32,
        pub current_window_start: BlockNumberFor<T>,
        pub current_window_proofs: u32,
        pub status: WindowStatus,
    }

    impl<T: Config> Default for LivenessTracker<T> {
        fn default() -> Self {
            Self {
                completed_windows: 0,
                current_window_start: Default::default(),
                current_window_proofs: 0,
                status: WindowStatus::Active,
            }
        }
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum WindowStatus {
        Active,
        Completed,
        Failed,
    }

    #[pallet::storage]
    #[pallet::getter(fn pallet_admin)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proof_chains)]
    pub type ProofChains<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<LivenessProof<T>, T::MaxProofLength>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn recent_headers)]
    pub type RecentHeaders<T: Config> = StorageValue<
        _,
        BoundedVec<HeaderData<T>, ConstU32<10>>, // Keep last 10 headers
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        WindowCompleted {
            who: T::AccountId,
            windows_completed: u32,
        },
        ParticipationFailed {
            who: T::AccountId,
        },
        ProofSubmitted {
            who: T::AccountId,
            block_number: BlockNumberFor<T>,
        },
        LivenessTrackingStarted {
            who: T::AccountId,
        },
        LivenessTrackingEnded {
            who: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        RestrictedAccess,
        NotTracking,
        ProofVerificationFailed,
        InvalidProofChain,
        MaxProofsExceeded,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_pallet_admin(origin: OriginFor<T>, new_admin: T::AccountId) -> DispatchResult {
            Self::root_or_admin(origin)?;
            PalletAdmin::<T>::put(new_admin);
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn submit_proof_unsigned(
            origin: OriginFor<T>,
            proof: LivenessProof<T>,
            who: T::AccountId,
        ) -> DispatchResult {
            ensure_none(origin)?;

            ensure!(
                ProofChains::<T>::contains_key(&who),
                Error::<T>::NotTracking
            );

            let current_block = <frame_system::Pallet<T>>::block_number();

            Self::validate_and_add_proof(&who, proof)?;

            Self::deposit_event(Event::ProofSubmitted {
                who,
                block_number: current_block,
            });

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block_number: BlockNumberFor<T>) {
            // Store header data for recently finalized blocks
            let header_data = HeaderData::<T> {
                block_number,
                block_hash: <frame_system::Pallet<T>>::block_hash(block_number),
                parent_hash: <frame_system::Pallet<T>>::parent_hash(),
                state_root: Default::default(),
                extrinsics_root: Default::default(),
            };

            RecentHeaders::<T>::mutate(|headers| {
                // Add new header, removing oldest if at capacity
                if headers.len() >= 10 {
                    headers.remove(0);
                }
                let _ = headers.try_push(header_data);
            });
        }

        fn offchain_worker(block_number: BlockNumberFor<T>) {
            if sp_io::offchain::is_validator() {
                return;
            }

            // Generate proof for each local authority key
            let authorities = Self::get_local_authority_keys();
            for authority in authorities {
                if let Some(account_id) = Self::authority_to_account(&authority) {
                    if ProofChains::<T>::contains_key(&account_id) {
                        let _ = Self::generate_and_submit_proof_offchain(
                            &account_id,
                            &authority,
                            block_number,
                        );
                    }
                }
            }
        }
    }

    impl<T: Config> PromotionCriteria<T::AccountId> for Pallet<T> {
        type CriteriaData = LivenessTracker<T>;
        type RegistrationData = pallet_d9_candidate_registry::SessionKeysOf<T>;

        fn validate_registration_data(_keys: &Self::RegistrationData) -> bool {
            // Validate that all keys are properly formatted
            // In a real implementation, you might check that keys are valid public keys
            true
        }

        fn initialize_criteria_data<B>(
            _who: &T::AccountId,
            _starting_block: B,
            _registration_data: &Self::RegistrationData,
        ) -> Self::CriteriaData {
            // For now, use the current block number
            let starting_block_number = <frame_system::Pallet<T>>::block_number();
            LivenessTracker {
                completed_windows: 0,
                current_window_start: starting_block_number,
                current_window_proofs: 0,
                status: WindowStatus::Active,
            }
        }

        fn update_participation_data<B>(
            who: &T::AccountId,
            data: &mut Self::CriteriaData,
            _current_block: B,
        ) -> DispatchResult {
            let current_block_number = <frame_system::Pallet<T>>::block_number();
            if Self::is_window_complete(data, current_block_number) {
                if data.current_window_proofs >= T::MinWindowThreshold::get() {
                    data.completed_windows = data.completed_windows.saturating_add(1);
                    data.status = WindowStatus::Completed;

                    Self::deposit_event(Event::WindowCompleted {
                        who: who.clone(),
                        windows_completed: data.completed_windows,
                    });
                } else {
                    data.status = WindowStatus::Failed;
                    Self::deposit_event(Event::ParticipationFailed { who: who.clone() });
                }

                Self::advance_to_next_window(data, current_block_number);
            }

            Ok(())
        }

        fn evaluate_promotion_eligibility(_who: &T::AccountId, data: &Self::CriteriaData) -> bool {
            data.completed_windows >= T::MinQualifyingWindows::get()
        }

        fn cleanup_criteria_data(who: &T::AccountId) {
            ProofChains::<T>::remove(who);
        }

        fn on_participation_started(who: &T::AccountId, _data: &Self::CriteriaData) {
            ProofChains::<T>::insert(who, BoundedVec::new());
            Self::deposit_event(Event::LivenessTrackingStarted { who: who.clone() });
        }

        fn on_participation_ended(who: &T::AccountId, _data: &Self::CriteriaData) {
            Self::deposit_event(Event::LivenessTrackingEnded { who: who.clone() });
        }
    }

    impl<T: Config> Pallet<T> {
        fn root_or_admin(origin: OriginFor<T>) -> Result<(), BadOrigin> {
            let origin = ensure_signed_or_root(origin)?;
            match origin {
                Some(caller) => {
                    let admin = PalletAdmin::<T>::get();
                    if admin.is_some() && admin.unwrap() == caller {
                        return Ok(());
                    } else {
                        return Err(BadOrigin);
                    }
                }
                None => {
                    return Ok(());
                }
            }
        }

        fn get_local_authority_keys() -> Vec<<T as Config>::AuthorityId> {
            // In a real implementation, this would get local authority keys
            // For now, return empty vec
            Vec::new()
        }

        fn authority_to_account(_authority: &<T as Config>::AuthorityId) -> Option<T::AccountId> {
            // Convert authority public key to account ID
            // This depends on the specific crypto scheme used
            None // Implementation depends on runtime configuration
        }

        fn generate_and_submit_proof_offchain(
            who: &T::AccountId,
            _authority: &<T as Config>::AuthorityId,
            block_number: BlockNumberFor<T>,
        ) -> Result<(), Error<T>> {
            // Create storage lock to prevent concurrent proof generation
            let mut lock =
                StorageLock::<BlockAndTime<frame_system::Pallet<T>>>::with_block_and_time_deadline(
                    b"liveness_proof_lock",
                    1,
                    sp_runtime::offchain::Duration::from_millis(2000),
                );

            if let Ok(_guard) = lock.try_lock() {
                let block_hash = <frame_system::Pallet<T>>::block_hash(block_number);
                // let header = <frame_system::Pallet<T>>::finalized_head();

                // Get node-specific data that proves we're actually running
                let parent_hash = if block_number > 1u32.into() {
                    <frame_system::Pallet<T>>::block_hash(block_number - 1u32.into())
                } else {
                    Default::default()
                };

                // Access block header data that only a synced node would have
                let state_root: T::Hash = Default::default();
                let extrinsics_root: T::Hash = Default::default();
                let extrinsic_count = <frame_system::Pallet<T>>::extrinsic_count();

                let previous_proof = Self::get_latest_proof(who);

                // Create payload to sign: all the node-specific data
                let mut payload = block_hash.encode();
                payload.extend_from_slice(&parent_hash.encode());
                payload.extend_from_slice(&state_root.encode());
                payload.extend_from_slice(&extrinsics_root.encode());
                payload.extend_from_slice(&extrinsic_count.encode());
                if let Some(prev_sig) = previous_proof.as_ref().map(|p| &p.signature) {
                    payload.extend_from_slice(&prev_sig.encode());
                }

                // Sign with authority key
                // TODO: Implement actual signing with authority key
                let signature = BoundedVec::try_from(vec![0u8; 64]).unwrap_or_default();
                let proof = LivenessProof::<T> {
                    block_hash,
                    block_number,
                    parent_hash,
                    state_root,
                    extrinsics_root,
                    extrinsic_count,
                    previous_signature: previous_proof.map(|p| p.signature),
                    signature,
                };

                // Submit unsigned transaction
                let _call = Call::submit_proof_unsigned {
                    proof: proof.clone(),
                    who: who.clone(),
                };
                // TODO: Implement offchain transaction submission
                // let _ = offchain::submit_transaction(call.into());
            }

            Ok(())
        }

        fn generate_and_submit_proof(
            who: &T::AccountId,
            block_number: BlockNumberFor<T>,
        ) -> Result<(), Error<T>> {
            let block_hash = <frame_system::Pallet<T>>::block_hash(block_number);
            let parent_hash = if block_number > 1u32.into() {
                <frame_system::Pallet<T>>::block_hash(block_number - 1u32.into())
            } else {
                Default::default()
            };
            let state_root: T::Hash = Default::default();
            let extrinsics_root: T::Hash = Default::default();
            let extrinsic_count = <frame_system::Pallet<T>>::extrinsic_count();
            let previous_proof = Self::get_latest_proof(who);

            let proof = LivenessProof {
                block_hash,
                block_number,
                parent_hash,
                state_root,
                extrinsics_root,
                extrinsic_count,
                previous_signature: previous_proof.map(|p| p.signature),
                signature: BoundedVec::try_from(vec![0u8; 64]).unwrap_or_default(),
            };

            Self::validate_and_add_proof(who, proof).map_err(|_| Error::<T>::ProofVerificationFailed)?;
            Ok(())
        }

        fn validate_and_add_proof(who: &T::AccountId, proof: LivenessProof<T>) -> DispatchResult {
            // Validate node-specific data against current chain state
            ensure!(
                Self::validate_node_data(&proof),
                Error::<T>::ProofVerificationFailed
            );

            ensure!(
                Self::validate_proof_signature(&proof, who),
                Error::<T>::ProofVerificationFailed
            );

            let previous_proof = Self::get_latest_proof(who);
            ensure!(
                Self::validate_proof_chain_continuity(&proof, previous_proof.as_ref()),
                Error::<T>::InvalidProofChain
            );

            Self::add_proof_to_chain(who, proof)?;

            Ok(())
        }

        fn validate_node_data(proof: &LivenessProof<T>) -> bool {
            // Find the header data for the block in the proof
            let headers = RecentHeaders::<T>::get();
            let header = headers
                .iter()
                .find(|h| h.block_number == proof.block_number);

            if let Some(header) = header {
                // Verify all node-specific data against stored header
                if proof.block_hash != header.block_hash {
                    return false;
                }

                if proof.parent_hash != header.parent_hash {
                    return false;
                }

                if proof.state_root != header.state_root {
                    return false;
                }

                if proof.extrinsics_root != header.extrinsics_root {
                    return false;
                }

                true
            } else {
                // Header not found in recent headers - proof is too old
                false
            }
        }

        fn validate_proof_signature(_proof: &LivenessProof<T>, _who: &T::AccountId) -> bool {
            // In real implementation, verify the signature was made with the authority's session key
            true
        }

        fn validate_proof_chain_continuity(
            new_proof: &LivenessProof<T>,
            previous_proof: Option<&LivenessProof<T>>,
        ) -> bool {
            match (previous_proof, &new_proof.previous_signature) {
                (Some(prev), Some(prev_sig)) => &prev.signature == prev_sig,
                (None, None) => true,
                _ => false,
            }
        }

        fn get_latest_proof(who: &T::AccountId) -> Option<LivenessProof<T>> {
            ProofChains::<T>::get(who)?.last().cloned()
        }

        fn add_proof_to_chain(who: &T::AccountId, proof: LivenessProof<T>) -> DispatchResult {
            ProofChains::<T>::try_mutate(who, |proofs_opt| {
                let proofs = proofs_opt.as_mut().ok_or(Error::<T>::NotTracking)?;

                if proofs.len() >= T::MaxProofLength::get() as usize {
                    proofs.remove(0);
                }

                proofs
                    .try_push(proof)
                    .map_err(|_| Error::<T>::MaxProofsExceeded)?;

                Ok(())
            })
        }

        fn is_window_complete(
            tracker: &LivenessTracker<T>,
            current_block: BlockNumberFor<T>,
        ) -> bool {
            current_block
                >= tracker
                    .current_window_start
                    .saturating_add(T::WindowSizeBlocks::get())
        }

        fn advance_to_next_window(
            tracker: &mut LivenessTracker<T>,
            current_block: BlockNumberFor<T>,
        ) {
            tracker.current_window_start = current_block;
            tracker.current_window_proofs = 0;
            tracker.status = WindowStatus::Active;
        }

        pub fn get_proof_count(who: &T::AccountId) -> u32 {
            ProofChains::<T>::get(who).map_or(0, |proofs| proofs.len() as u32)
        }

        pub fn verify_proof_chain(who: &T::AccountId) -> bool {
            if let Some(proofs) = ProofChains::<T>::get(who) {
                for window in proofs.windows(2) {
                    if let [prev, curr] = window {
                        if curr.previous_signature != Some(prev.signature.clone()) {
                            return false;
                        }
                    }
                }
                true
            } else {
                false
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::submit_proof_unsigned { proof, who } => {
                    // First validate node-specific data
                    if !Self::validate_node_data(proof) {
                        return InvalidTransaction::BadProof.into();
                    }

                    // Validate proof signature and chain continuity
                    if !Self::validate_proof_signature(proof, who) {
                        return InvalidTransaction::BadProof.into();
                    }

                    let previous_proof = Self::get_latest_proof(who);
                    if !Self::validate_proof_chain_continuity(proof, previous_proof.as_ref()) {
                        return InvalidTransaction::BadProof.into();
                    }

                    // Ensure the account is being tracked
                    if !ProofChains::<T>::contains_key(who) {
                        return InvalidTransaction::Custom(1).into();
                    }

                    ValidTransaction::with_tag_prefix("LivenessProof")
                        .priority(TransactionPriority::max_value())
                        .and_provides(vec![(who, proof.block_number).encode()])
                        .longevity(3)
                        .propagate(true)
                        .build()
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}
