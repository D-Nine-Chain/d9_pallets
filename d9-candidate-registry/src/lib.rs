#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use frame_support::{pallet_prelude::*, traits::Get, weights::Weight};
use frame_system::pallet_prelude::*;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
    traits::{BadOrigin, SaturatedConversion},
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        ValidTransaction,
    },
};
use sp_std::vec::Vec;

pub use pallet::*;

// Import traits and types from primitives
use pallet_d9_node_primitives::{
    PromotionCriteria, CandidateManager,
    ValidationChallenge, ValidationFailureReason,
    RegistryEventHandler, VotingEventHandler
};

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// ImOnlineId used by liveness check, the others are signature validated
    /// to make sure they setup the node correctly.
    pub type SessionKeysOf<T> = pallet_d9_node_primitives::SessionKeys<
        <T as Config>::AuraId,
        <T as Config>::AuthorityId,
        <T as Config>::GrandpaId,
        <T as Config>::ImOnlineId,
    >;

    pub type ValidationChallengeOf<T> =
        ValidationChallenge<<T as frame_system::Config>::Hash, BlockNumberFor<T>>;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_session::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The promotion criteria implementation
        type PromotionCriteria: PromotionCriteria<
            Self::AccountId,
            RegistrationData = SessionKeysOf<Self>,
        >;

        /// Metadata type for candidates
        type CandidateMetadata: Parameter + Member + Default + MaxEncodedLen;

        /// Maximum time for key validation (in blocks)
        #[pallet::constant]
        type ValidationTimeout: Get<BlockNumberFor<Self>>;

        /// Authority ID types for session keys
        type AuraId: Member
            + Parameter
            + RuntimeAppPublic
            + MaybeSerializeDeserialize
            + MaxEncodedLen;
        type AuthorityId: Member
            + Parameter
            + RuntimeAppPublic
            + MaybeSerializeDeserialize
            + MaxEncodedLen;
        type GrandpaId: Member
            + Parameter
            + RuntimeAppPublic
            + MaybeSerializeDeserialize
            + MaxEncodedLen;
        type ImOnlineId: Member
            + Parameter
            + RuntimeAppPublic
            + MaybeSerializeDeserialize
            + MaxEncodedLen;

        /// Handler for registry events
        type RegistryEventHandler: RegistryEventHandler<Self::AccountId>;
    }

    #[pallet::storage]
    #[pallet::getter(fn pallet_admin)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn aspirant_data)]
    pub type AspirantData<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        <T::PromotionCriteria as PromotionCriteria<T::AccountId>>::CriteriaData,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn pending_validations)]
    pub type PendingValidations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        (SessionKeysOf<T>, ValidationChallengeOf<T>),
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn validated_aspirants)]
    pub type ValidatedAspirants<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, SessionKeysOf<T>, OptionQuery>;

    #[pallet::storage]
    pub type Nonce<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Tracks all active candidates
    #[pallet::storage]
    #[pallet::getter(fn candidates)]
    pub type Candidates<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::CandidateMetadata, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AspirantJoined {
            who: T::AccountId,
        },
        AspirantLeft {
            who: T::AccountId,
        },
        PromotedToCandidate {
            who: T::AccountId,
        },
        PromotionCriteriaChanged,
        AllAspirantDataCleared,
        KeysSubmitted {
            who: T::AccountId,
        },
        ValidationRequested {
            who: T::AccountId,
        },
        KeysValidated {
            who: T::AccountId,
        },
        ValidationFailed {
            who: T::AccountId,
            reason: ValidationFailureReason,
        },
        ValidationExpired {
            who: T::AccountId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        RestrictedAccess,
        AlreadyAspirant,
        NotAspirant,
        PromotionCriteriaError,
        CandidateManagerError,
        AlreadyPendingValidation,
        NoPendingValidation,
        InvalidSignatures,
        ChallengeExpired,
        InvalidSessionKeys,
        AlreadyValidated,
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

        /// origin is aspirant (assumed reward recipient)
        /// session_keys assumed to be swappable
        /// which means one can change sessions keys and
        /// not affect node rewards.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn become_aspirant(
            origin: OriginFor<T>,
            session_keys: SessionKeysOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !AspirantData::<T>::contains_key(&who),
                Error::<T>::AlreadyAspirant
            );
            ensure!(
                !PendingValidations::<T>::contains_key(&who),
                Error::<T>::AlreadyPendingValidation
            );
            ensure!(
                !ValidatedAspirants::<T>::contains_key(&who),
                Error::<T>::AlreadyValidated
            );

            let registration_data = session_keys.clone();
            ensure!(
                T::PromotionCriteria::validate_registration_data(&registration_data),
                Error::<T>::InvalidSessionKeys
            );

            let challenge = Self::generate_validation_challenge();
            PendingValidations::<T>::insert(&who, (&session_keys, &challenge));

            Self::deposit_event(Event::KeysSubmitted { who: who.clone() });
            Self::deposit_event(Event::ValidationRequested { who });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn leave_aspirant_pool(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let criteria_data = AspirantData::<T>::get(&who).ok_or(Error::<T>::NotAspirant)?;

            AspirantData::<T>::remove(&who);
            ValidatedAspirants::<T>::remove(&who);
            
            // Notify handler before cleanup
            T::RegistryEventHandler::on_aspirant_removed(&who);
            
            T::PromotionCriteria::on_participation_ended(&who, &criteria_data);
            T::PromotionCriteria::cleanup_criteria_data(&who);

            Self::deposit_event(Event::AspirantLeft { who });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(0, u64::MAX))]
        pub fn clear_all_aspirant_data(origin: OriginFor<T>) -> DispatchResult {
            Self::root_or_admin(origin)?;

            let aspirants: Vec<T::AccountId> = AspirantData::<T>::iter_keys().collect();

            for aspirant in aspirants {
                if let Some(criteria_data) = AspirantData::<T>::get(&aspirant) {
                    T::PromotionCriteria::on_participation_ended(&aspirant, &criteria_data);
                    T::PromotionCriteria::cleanup_criteria_data(&aspirant);
                }
                AspirantData::<T>::remove(&aspirant);
                ValidatedAspirants::<T>::remove(&aspirant);
            }

            let _ = PendingValidations::<T>::clear(u32::MAX, None);

            Self::deposit_event(Event::AllAspirantDataCleared);
            Self::deposit_event(Event::PromotionCriteriaChanged);

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(4, 3))]
        pub fn submit_key_validation_proof(
            origin: OriginFor<T>,
            who: T::AccountId,
            aura_signature: <T::AuraId as RuntimeAppPublic>::Signature,
            authority_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
            grandpa_signature: <T::GrandpaId as RuntimeAppPublic>::Signature,
            im_online_signature: <T::ImOnlineId as RuntimeAppPublic>::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let (session_keys, challenge) =
                PendingValidations::<T>::get(&who).ok_or(Error::<T>::NoPendingValidation)?;

            ensure!(
                !Self::is_challenge_expired(&challenge),
                Error::<T>::ChallengeExpired
            );

            ensure!(
                Self::verify_all_signatures(
                    &session_keys,
                    &challenge,
                    &aura_signature,
                    &authority_signature,
                    &grandpa_signature,
                    &im_online_signature,
                ),
                Error::<T>::InvalidSignatures
            );

            Self::finalize_validation(&who, session_keys)?;

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::cleanup_expired_validations(n);
            Weight::from_parts(10_000, 0)
        }

        fn on_finalize(block_number: BlockNumberFor<T>) {
            Self::process_periodic_updates(block_number);
        }

        fn offchain_worker(block_number: BlockNumberFor<T>) {
            if let Err(e) = Self::offchain_key_validation(block_number) {
                log::error!("Error in offchain worker: {:?}", e);
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::submit_key_validation_proof {
                    who,
                    aura_signature,
                    authority_signature,
                    grandpa_signature,
                    im_online_signature,
                } => {
                    let (keys, challenge) =
                        PendingValidations::<T>::get(who).ok_or(InvalidTransaction::Custom(1))?;

                    if Self::is_challenge_expired(&challenge) {
                        return InvalidTransaction::Stale.into();
                    }

                    if !Self::verify_all_signatures(
                        &keys,
                        &challenge,
                        aura_signature,
                        authority_signature,
                        grandpa_signature,
                        im_online_signature,
                    ) {
                        return InvalidTransaction::BadProof.into();
                    }

                    ValidTransaction::with_tag_prefix("KeyValidation")
                        .priority(TransactionPriority::max_value())
                        .and_provides(vec![(who, challenge.nonce).encode()])
                        .longevity(T::ValidationTimeout::get().saturated_into())
                        .propagate(true)
                        .build()
                }
                _ => InvalidTransaction::Call.into(),
            }
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

        fn generate_validation_challenge() -> ValidationChallengeOf<T> {
            let current_block = <frame_system::Pallet<T>>::block_number();
            let block_hash = <frame_system::Pallet<T>>::block_hash(current_block);
            let nonce = Nonce::<T>::mutate(|n| {
                *n = n.saturating_add(1);
                *n
            });

            ValidationChallenge {
                block_hash,
                nonce,
                expires_at: current_block + T::ValidationTimeout::get(),
            }
        }

        fn is_challenge_expired(challenge: &ValidationChallengeOf<T>) -> bool {
            let current_block = <frame_system::Pallet<T>>::block_number();
            current_block > challenge.expires_at
        }

        fn get_challenge_payload(challenge: &ValidationChallengeOf<T>) -> Vec<u8> {
            let mut payload = Vec::new();
            payload.extend_from_slice(&challenge.block_hash.encode());
            payload.extend_from_slice(&challenge.nonce.encode());
            payload.extend_from_slice(&challenge.expires_at.encode());
            payload
        }

        fn verify_all_signatures(
            keys: &SessionKeysOf<T>,
            challenge: &ValidationChallengeOf<T>,
            aura_signature: &<T::AuraId as RuntimeAppPublic>::Signature,
            authority_signature: &<T::AuthorityId as RuntimeAppPublic>::Signature,
            grandpa_signature: &<T::GrandpaId as RuntimeAppPublic>::Signature,
            im_online_signature: &<T::ImOnlineId as RuntimeAppPublic>::Signature,
        ) -> bool {
            let payload = Self::get_challenge_payload(challenge);

            keys.aura.verify(&payload, aura_signature)
                && keys.authority.verify(&payload, authority_signature)
                && keys.grandpa.verify(&payload, grandpa_signature)
                && keys.im_online.verify(&payload, im_online_signature)
        }

        fn finalize_validation(who: &T::AccountId, keys: SessionKeysOf<T>) -> DispatchResult {
            PendingValidations::<T>::remove(who);
            ValidatedAspirants::<T>::insert(who, &keys);

            let current_block = <frame_system::Pallet<T>>::block_number();
            let criteria_data =
                T::PromotionCriteria::initialize_criteria_data(who, current_block, &keys);

            AspirantData::<T>::insert(who, &criteria_data);
            
            // Notify handler that keys are validated
            T::RegistryEventHandler::on_keys_validated(who, &keys.encode());
            
            T::PromotionCriteria::on_participation_started(who, &criteria_data);

            Self::deposit_event(Event::KeysValidated { who: who.clone() });
            Self::deposit_event(Event::AspirantJoined { who: who.clone() });

            Ok(())
        }

        fn cleanup_expired_validations(current_block: BlockNumberFor<T>) {
            let expired: Vec<T::AccountId> = PendingValidations::<T>::iter()
                .filter_map(|(who, (_, challenge))| {
                    if current_block > challenge.expires_at {
                        Some(who)
                    } else {
                        None
                    }
                })
                .collect();

            for who in expired {
                PendingValidations::<T>::remove(&who);
                Self::deposit_event(Event::ValidationExpired { who });
            }
        }

        fn offchain_key_validation(_block_number: BlockNumberFor<T>) -> Result<(), &'static str> {
            let local_keys = Self::get_local_session_keys()?;

            for (account, keys) in local_keys {
                if let Some((pending_keys, challenge)) = PendingValidations::<T>::get(&account) {
                    if pending_keys == keys && !Self::is_challenge_expired(&challenge) {
                        if let Ok(signatures) =
                            Self::sign_challenge_with_all_keys(&keys, &challenge)
                        {
                            let _call = Call::<T>::submit_key_validation_proof {
                                who: account,
                                aura_signature: signatures.0,
                                authority_signature: signatures.1,
                                grandpa_signature: signatures.2,
                                im_online_signature: signatures.3,
                            };

                            // TODO: Uncomment when SendTransactionTypes is added to Config
                            // let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
                        }
                    }
                }
            }

            Ok(())
        }

        fn get_local_session_keys() -> Result<Vec<(T::AccountId, SessionKeysOf<T>)>, &'static str> {
            // This would be implemented by the runtime to check local keys
            // For now, return empty vec
            Ok(Vec::new())
        }

        fn sign_challenge_with_all_keys(
            _keys: &SessionKeysOf<T>,
            _challenge: &ValidationChallengeOf<T>,
        ) -> Result<
            (
                <T::AuraId as RuntimeAppPublic>::Signature,
                <T::AuthorityId as RuntimeAppPublic>::Signature,
                <T::GrandpaId as RuntimeAppPublic>::Signature,
                <T::ImOnlineId as RuntimeAppPublic>::Signature,
            ),
            &'static str,
        > {
            // This would be implemented to sign with each key type
            Err("Not implemented")
        }

        fn promote_to_candidate(
            who: &T::AccountId,
            criteria_data: &<T::PromotionCriteria as PromotionCriteria<T::AccountId>>::CriteriaData,
        ) -> DispatchResult {
            // Use default metadata for now - in real use, this could come from criteria_data
            let metadata = T::CandidateMetadata::default();

            <Self as CandidateManager<T::AccountId, T::CandidateMetadata>>::add_candidate(
                who, metadata,
            )
            .map_err(|_| Error::<T>::CandidateManagerError)?;

            AspirantData::<T>::remove(who);
            ValidatedAspirants::<T>::remove(who);
            T::PromotionCriteria::on_participation_ended(who, criteria_data);
            T::PromotionCriteria::cleanup_criteria_data(who);

            Self::deposit_event(Event::PromotedToCandidate { who: who.clone() });

            Ok(())
        }

        fn process_periodic_updates(block_number: BlockNumberFor<T>) {
            let aspirants: Vec<(
                T::AccountId,
                <T::PromotionCriteria as PromotionCriteria<T::AccountId>>::CriteriaData,
            )> = AspirantData::<T>::iter().collect();

            for (who, mut criteria_data) in aspirants {
                if let Ok(()) = T::PromotionCriteria::update_participation_data(
                    &who,
                    &mut criteria_data,
                    block_number,
                ) {
                    let should_promote =
                        T::PromotionCriteria::evaluate_promotion_eligibility(&who, &criteria_data);

                    AspirantData::<T>::insert(&who, &criteria_data);

                    if should_promote {
                        let _ = Self::promote_to_candidate(&who, &criteria_data);
                    }
                }
            }
        }

        pub fn is_aspirant(who: &T::AccountId) -> bool {
            AspirantData::<T>::contains_key(who)
        }

        pub fn get_aspirant_data(
            who: &T::AccountId,
        ) -> Option<<T::PromotionCriteria as PromotionCriteria<T::AccountId>>::CriteriaData>
        {
            AspirantData::<T>::get(who)
        }
    }

    // Implement CandidateManager
    impl<T: Config> CandidateManager<T::AccountId, T::CandidateMetadata> for Pallet<T> {
        fn add_candidate(who: &T::AccountId, metadata: T::CandidateMetadata) -> DispatchResult {
            // Store in our registry
            Candidates::<T>::insert(who, metadata.clone());

            // Notify node-voting pallet to add this candidate
            // Note: This requires the runtime to implement the necessary trait binding
            // In runtime configuration, CandidateMetadata should be NodeMetadataStruct

            Ok(())
        }

        fn remove_candidate(who: &T::AccountId) -> DispatchResult {
            Candidates::<T>::remove(who);

            // Notify node-voting pallet to remove this candidate
            // Note: This requires runtime configuration

            Ok(())
        }

        fn is_candidate(who: &T::AccountId) -> bool {
            Candidates::<T>::contains_key(who)
        }
    }

    // Implement handler for voting events
    impl<T: Config> VotingEventHandler<T::AccountId> for Pallet<T> {
        fn on_candidate_removed(who: &T::AccountId) {
            // Clean up any aspirant data if they were removed from voting
            AspirantData::<T>::remove(who);
            ValidatedAspirants::<T>::remove(who);
        }
        
        fn on_candidate_added(_who: &T::AccountId) {
            // No action needed
        }
    }
}
