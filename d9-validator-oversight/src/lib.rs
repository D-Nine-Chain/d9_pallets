#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use pallet_d9_punishment::{PenaltyReporter, PenaltyType, Severity};
use sp_runtime::traits::{Convert, Saturating, Zero};
use sp_staking::SessionIndex;
use sp_std::vec::Vec;

pub use pallet::*;

/// Types of violations that can be detected
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ViolationType {
    /// Validator was offline for too long
    Offline { missed_blocks: u32 },
    /// Validator equivocated (double-signed)
    Equivocation,
    /// Validator failed to produce blocks when scheduled
    MissedBlocks { count: u32 },
    /// Custom violation type
    Custom(BoundedVec<u8, ConstU32<32>>),
}

/// Violation record
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct ViolationRecord<T: Config> {
    /// Type of violation
    pub violation_type: ViolationType,
    /// When the violation occurred
    pub occurred_at: BlockNumberFor<T>,
    /// Session when violation occurred
    pub session: SessionIndex,
}

/// Validator monitoring status
#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct ValidatorStatus {
    /// Number of blocks validated
    pub blocks_validated: u32,
    /// Number of blocks missed
    pub blocks_missed: u32,
    /// Last active block
    pub last_active: Option<u32>,
    /// Total violations count
    pub violation_count: u32,
}

impl Default for ValidatorStatus {
    fn default() -> Self {
        Self {
            blocks_validated: 0,
            blocks_missed: 0,
            last_active: None,
            violation_count: 0,
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_session::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Penalty reporter for reporting violations
        type PenaltyReporter: PenaltyReporter<Self::AccountId, BlockNumberFor<Self>>;
        
        /// Convert ValidatorId to AccountId
        type ValidatorIdOf: Convert<Self::ValidatorId, Option<Self::AccountId>>;

        /// Maximum violations to track per validator
        #[pallet::constant]
        type MaxViolationsPerValidator: Get<u32>;

        /// Number of blocks before validator is considered offline
        #[pallet::constant]
        type OfflineThreshold: Get<u32>;

        /// Number of missed blocks before issuing penalty
        #[pallet::constant]
        type MissedBlocksThreshold: Get<u32>;

        /// Duration of penalties for different violations (in blocks)
        #[pallet::constant]
        type OfflinePenaltyDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type EquivocationPenaltyDuration: Get<BlockNumberFor<Self>>;

        /// Window size for tracking violations
        #[pallet::constant]
        type MonitoringWindow: Get<BlockNumberFor<Self>>;
    }

    /// Current session's active validators
    #[pallet::storage]
    #[pallet::getter(fn active_validators)]
    pub type ActiveValidators<T: Config> = StorageValue<
        _,
        BoundedVec<T::AccountId, ConstU32<1000>>,
        ValueQuery,
    >;

    /// Validator monitoring status
    #[pallet::storage]
    #[pallet::getter(fn validator_status)]
    pub type ValidatorStatuses<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ValidatorStatus,
        ValueQuery,
    >;

    /// Violations per validator
    #[pallet::storage]
    #[pallet::getter(fn validator_violations)]
    pub type ValidatorViolations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<ViolationRecord<T>, T::MaxViolationsPerValidator>,
        ValueQuery,
    >;

    /// Last processed block for cleanup
    #[pallet::storage]
    pub type LastProcessedBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Penalty IDs issued by this pallet (for tracking/revocation)
    #[pallet::storage]
    #[pallet::getter(fn issued_penalties)]
    pub type IssuedPenalties<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        ViolationType,
        <T::PenaltyReporter as PenaltyReporter<T::AccountId, BlockNumberFor<T>>>::PenaltyId,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New validators set for monitoring
        ValidatorsUpdated {
            count: u32,
        },
        /// Validator violation detected
        ViolationDetected {
            validator: T::AccountId,
            violation_type: ViolationType,
        },
        /// Penalty issued for violation
        PenaltyIssued {
            validator: T::AccountId,
            violation_type: ViolationType,
            penalty_id: <T::PenaltyReporter as PenaltyReporter<T::AccountId, BlockNumberFor<T>>>::PenaltyId,
        },
        /// Validator came back online
        ValidatorRecovered {
            validator: T::AccountId,
        },
        /// Old violation data cleaned up
        ViolationsCleanedUp {
            count: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not a current validator
        NotActiveValidator,
        /// Violation already reported
        DuplicateViolation,
        /// Failed to issue penalty
        PenaltyIssuanceFailed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Report that a validator is offline
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
        pub fn report_offline(
            origin: OriginFor<T>,
            validator: T::AccountId,
        ) -> DispatchResult {
            // This could be restricted to certain origins (e.g., other validators)
            ensure_signed(origin)?;
            
            ensure!(
                Self::is_active_validator(&validator),
                Error::<T>::NotActiveValidator
            );

            let current_block = frame_system::Pallet::<T>::block_number();
            let missed_blocks = Self::calculate_missed_blocks(&validator, current_block);
            
            if missed_blocks >= T::OfflineThreshold::get() {
                let violation = ViolationType::Offline { missed_blocks };
                Self::record_violation(&validator, violation.clone())?;
                Self::issue_penalty_for_violation(&validator, violation)?;
            }

            Ok(())
        }

        /// Report equivocation (double-signing)
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
        pub fn report_equivocation(
            origin: OriginFor<T>,
            validator: T::AccountId,
            _evidence: Vec<u8>, // In real implementation, this would be cryptographic proof
        ) -> DispatchResult {
            // This should be restricted and verify evidence
            ensure_root(origin)?;
            
            ensure!(
                Self::is_active_validator(&validator),
                Error::<T>::NotActiveValidator
            );

            let violation = ViolationType::Equivocation;
            Self::record_violation(&validator, violation.clone())?;
            Self::issue_penalty_for_violation(&validator, violation)?;

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads(1);

            // Update validator statuses periodically
            if now % 100u32.into() == Zero::zero() {
                weight = weight.saturating_add(Self::update_validator_statuses(now));
            }

            // Clean up old violations
            if now % 1000u32.into() == Zero::zero() {
                weight = weight.saturating_add(Self::cleanup_old_violations(now));
            }

            weight
        }
    }

    // Session manager integration
    impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
        fn new_session(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
            // Let the next session manager in the chain handle the actual session logic
            None
        }

        fn end_session(_end_index: SessionIndex) {}

        fn start_session(_start_index: SessionIndex) {
            // Update our active validator set when a new session starts
            let validators = pallet_session::Pallet::<T>::validators();
            
            // Convert ValidatorId to AccountId
            let account_ids: Vec<T::AccountId> = validators.iter()
                .filter_map(|v| <T as Config>::ValidatorIdOf::convert(v.clone()))
                .collect();
            
            let bounded_validators = BoundedVec::try_from(account_ids)
                .unwrap_or_else(|_| BoundedVec::new());
            
            ActiveValidators::<T>::put(bounded_validators.clone());
            
            Self::deposit_event(Event::ValidatorsUpdated {
                count: bounded_validators.len() as u32,
            });
        }
    }

    impl<T: Config> Pallet<T> {
        /// Check if account is active validator
        fn is_active_validator(who: &T::AccountId) -> bool {
            ActiveValidators::<T>::get().contains(who)
        }

        /// Calculate blocks missed by validator
        fn calculate_missed_blocks(validator: &T::AccountId, current_block: BlockNumberFor<T>) -> u32 {
            let status = ValidatorStatuses::<T>::get(validator);
            
            if let Some(last_active_u32) = status.last_active {
                let last_active: BlockNumberFor<T> = last_active_u32.into();
                let blocks_since = current_block.saturating_sub(last_active);
                
                // Convert to u32, saturating at u32::MAX if needed
                blocks_since.try_into().unwrap_or(u32::MAX)
            } else {
                u32::MAX // Never been active
            }
        }

        /// Record a violation
        fn record_violation(
            validator: &T::AccountId,
            violation_type: ViolationType,
        ) -> DispatchResult {
            let current_block = frame_system::Pallet::<T>::block_number();
            let session = pallet_session::Pallet::<T>::current_index();

            let violation = ViolationRecord {
                violation_type: violation_type.clone(),
                occurred_at: current_block,
                session,
            };

            ValidatorViolations::<T>::try_mutate(validator, |violations| {
                // Check for duplicate recent violations
                let duplicate = violations.iter().any(|v| {
                    v.violation_type == violation_type &&
                    v.occurred_at > current_block.saturating_sub(100u32.into())
                });

                if duplicate {
                    return Err(Error::<T>::DuplicateViolation);
                }

                if violations.try_push(violation.clone()).is_err() {
                    // If full, remove oldest and try again
                    violations.remove(0);
                    violations.try_push(violation).map_err(|_| Error::<T>::DuplicateViolation)?;
                }
                
                Ok(())
            })?;

            // Update status
            ValidatorStatuses::<T>::mutate(validator, |status| {
                status.violation_count = status.violation_count.saturating_add(1);
            });

            Self::deposit_event(Event::ViolationDetected {
                validator: validator.clone(),
                violation_type,
            });

            Ok(())
        }

        /// Issue penalty for violation
        fn issue_penalty_for_violation(
            validator: &T::AccountId,
            violation_type: ViolationType,
        ) -> DispatchResult {
            let (penalty_type, severity, duration) = match &violation_type {
                ViolationType::Offline { missed_blocks } => {
                    let severity = if *missed_blocks > 1000 {
                        Severity::High
                    } else if *missed_blocks > 100 {
                        Severity::Medium
                    } else {
                        Severity::Low
                    };
                    
                    (
                        PenaltyType::ValidatorOffline,
                        severity,
                        Some(T::OfflinePenaltyDuration::get()),
                    )
                },
                ViolationType::Equivocation => (
                    PenaltyType::Equivocation,
                    Severity::Critical,
                    Some(T::EquivocationPenaltyDuration::get()),
                ),
                ViolationType::MissedBlocks { count } => {
                    let severity = if *count > 50 {
                        Severity::Medium
                    } else {
                        Severity::Low
                    };
                    
                    (
                        PenaltyType::ValidatorOffline,
                        severity,
                        Some(T::OfflinePenaltyDuration::get()),
                    )
                },
                ViolationType::Custom(data) => (
                    PenaltyType::Custom(data.clone()),
                    Severity::Medium,
                    Some(T::OfflinePenaltyDuration::get()),
                ),
            };

            match T::PenaltyReporter::report_penalty(
                validator,
                penalty_type,
                severity,
                duration,
            ) {
                Ok(penalty_id) => {
                    IssuedPenalties::<T>::insert(validator, &violation_type, penalty_id);
                    
                    Self::deposit_event(Event::PenaltyIssued {
                        validator: validator.clone(),
                        violation_type,
                        penalty_id,
                    });
                    
                    Ok(())
                },
                Err(_) => Err(Error::<T>::PenaltyIssuanceFailed.into()),
            }
        }

        /// Update validator statuses
        fn update_validator_statuses(now: BlockNumberFor<T>) -> Weight {
            let mut reads = 1u64;
            let mut writes = 0u64;

            let validators = ActiveValidators::<T>::get();
            reads = reads.saturating_add(1);

            for validator in validators.iter() {
                ValidatorStatuses::<T>::mutate(validator, |status| {
                    // In real implementation, this would check actual block production
                    // For now, we'll just update the last active time
                    status.last_active = Some(now.try_into().unwrap_or(u32::MAX));
                });
                writes = writes.saturating_add(1);
            }

            T::DbWeight::get().reads_writes(reads, writes)
        }

        /// Clean up old violations outside monitoring window
        fn cleanup_old_violations(now: BlockNumberFor<T>) -> Weight {
            let mut reads = 0u64;
            let mut writes = 0u64;
            let mut cleaned = 0u32;

            let cutoff = now.saturating_sub(T::MonitoringWindow::get());

            for (validator, mut violations) in ValidatorViolations::<T>::iter() {
                reads = reads.saturating_add(1);
                
                let original_len = violations.len();
                violations.retain(|v| v.occurred_at > cutoff);
                
                if violations.len() < original_len {
                    cleaned = cleaned.saturating_add((original_len - violations.len()) as u32);
                    
                    if violations.is_empty() {
                        ValidatorViolations::<T>::remove(&validator);
                    } else {
                        ValidatorViolations::<T>::insert(&validator, violations);
                    }
                    writes = writes.saturating_add(1);
                }
            }

            if cleaned > 0 {
                Self::deposit_event(Event::ViolationsCleanedUp { count: cleaned });
            }

            T::DbWeight::get().reads_writes(reads, writes)
        }
    }
}