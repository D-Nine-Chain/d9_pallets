#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_std::vec::Vec;

pub use pallet::*;

#[cfg(test)]
mod tests;

/// Trait for pallets to report penalties
pub trait PenaltyReporter<AccountId, BlockNumber> {
    type PenaltyId: Parameter + Member + Copy + MaxEncodedLen;

    /// Report a new penalty
    fn report_penalty(
        who: &AccountId,
        penalty_type: PenaltyType,
        severity: Severity,
        duration: Option<BlockNumber>,
    ) -> Result<Self::PenaltyId, DispatchError>;

    /// Revoke an existing penalty
    fn revoke_penalty(penalty_id: Self::PenaltyId) -> DispatchResult;
}

/// Trait for checking reward restrictions
pub trait RewardRestriction<AccountId> {
    /// Check if account has any restrictions
    fn has_restriction(who: &AccountId) -> bool;
    
    /// Get detailed restriction information
    fn get_restriction(who: &AccountId) -> RestrictionInfo;
}

/// Types of penalties
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PenaltyType {
    /// Governance-based account lock
    GovernanceLock,
    /// Validator was offline
    ValidatorOffline,
    /// Validator equivocated (double-signed)
    Equivocation,
    /// Custom penalty type for extensibility
    Custom(BoundedVec<u8, ConstU32<32>>),
}

/// Severity levels for penalties
#[derive(Encode, Decode, Clone, PartialEq, Eq, PartialOrd, Ord, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Severity {
    /// Minor infractions - small reward reduction
    Low,
    /// Moderate violations - significant reward reduction
    Medium,
    /// Severe violations - major reward reduction or blocking
    High,
    /// Critical violations - complete blocking
    Critical,
}

/// Information about active restrictions
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct RestrictionInfo {
    /// Whether rewards are completely blocked
    pub is_blocked: bool,
    /// Percentage of reward reduction (0-100)
    pub reduction_percent: u8,
    /// Number of active penalties
    pub active_penalty_count: u32,
    /// Highest severity among active penalties
    pub max_severity: Option<Severity>,
}

/// Record of a penalty
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PenaltyRecord<T: Config> {
    /// Unique penalty ID
    pub id: T::PenaltyId,
    /// Type of penalty
    pub penalty_type: PenaltyType,
    /// Severity level
    pub severity: Severity,
    /// When the penalty was issued
    pub issued_at: BlockNumberFor<T>,
    /// When the penalty expires (None = permanent until revoked)
    pub expires_at: Option<BlockNumberFor<T>>,
    /// Who reported this penalty
    pub reporter: T::AccountId,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type for penalty IDs
        type PenaltyId: Parameter + Member + Copy + Default + From<u64> + Into<u64> + MaxEncodedLen;

        /// Maximum penalties that can be active per account
        #[pallet::constant]
        type MaxPenaltiesPerAccount: Get<u32>;

        /// How long to keep penalty history after expiration
        #[pallet::constant]
        type PenaltyHistoryRetention: Get<BlockNumberFor<Self>>;
    }

    /// Counter for generating unique penalty IDs
    #[pallet::storage]
    pub type NextPenaltyId<T: Config> = StorageValue<_, T::PenaltyId, ValueQuery>;

    /// Active penalties per account
    #[pallet::storage]
    #[pallet::getter(fn active_penalties)]
    pub type ActivePenalties<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<PenaltyRecord<T>, T::MaxPenaltiesPerAccount>,
        ValueQuery,
    >;

    /// Penalty details by ID
    #[pallet::storage]
    #[pallet::getter(fn penalty_details)]
    pub type PenaltyDetails<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::PenaltyId,
        (T::AccountId, PenaltyRecord<T>),
        OptionQuery,
    >;

    /// Authorized penalty reporters
    #[pallet::storage]
    #[pallet::getter(fn authorized_reporters)]
    pub type AuthorizedReporters<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new penalty was issued
        PenaltyIssued {
            penalty_id: T::PenaltyId,
            who: T::AccountId,
            penalty_type: PenaltyType,
            severity: Severity,
            reporter: T::AccountId,
        },
        /// A penalty was revoked
        PenaltyRevoked {
            penalty_id: T::PenaltyId,
            who: T::AccountId,
        },
        /// A penalty expired
        PenaltyExpired {
            penalty_id: T::PenaltyId,
            who: T::AccountId,
        },
        /// Reporter authorization changed
        ReporterAuthorizationChanged {
            reporter: T::AccountId,
            authorized: bool,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Caller is not authorized to report penalties
        UnauthorizedReporter,
        /// Penalty ID not found
        PenaltyNotFound,
        /// Too many active penalties for account
        TooManyPenalties,
        /// Invalid duration specified
        InvalidDuration,
        /// Cannot revoke a permanent penalty
        CannotRevokePermanentPenalty,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Authorize or deauthorize a penalty reporter
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_reporter_authorization(
            origin: OriginFor<T>,
            reporter: T::AccountId,
            authorized: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;
            
            if authorized {
                AuthorizedReporters::<T>::insert(&reporter, true);
            } else {
                AuthorizedReporters::<T>::remove(&reporter);
            }
            
            Self::deposit_event(Event::ReporterAuthorizationChanged { reporter, authorized });
            Ok(())
        }

        /// Report a penalty (only callable by authorized reporters)
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
        pub fn report_penalty(
            origin: OriginFor<T>,
            who: T::AccountId,
            penalty_type: PenaltyType,
            severity: Severity,
            duration: Option<BlockNumberFor<T>>,
        ) -> DispatchResult {
            let reporter = ensure_signed(origin)?;
            ensure!(
                AuthorizedReporters::<T>::get(&reporter),
                Error::<T>::UnauthorizedReporter
            );

            let _penalty_id = Self::issue_penalty(&who, penalty_type, severity, duration, reporter)?;
            
            Ok(())
        }

        /// Revoke a penalty (only callable by the original reporter)
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn revoke_penalty(
            origin: OriginFor<T>,
            penalty_id: T::PenaltyId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            
            let (account, penalty) = PenaltyDetails::<T>::get(penalty_id)
                .ok_or(Error::<T>::PenaltyNotFound)?;
            
            // Only the reporter can revoke (or root in the future)
            ensure!(
                penalty.reporter == caller,
                Error::<T>::UnauthorizedReporter
            );
            
            Self::remove_penalty(&account, penalty_id)?;
            
            Self::deposit_event(Event::PenaltyRevoked {
                penalty_id,
                who: account,
            });
            
            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            Self::process_expired_penalties(now)
        }
    }

    impl<T: Config> Pallet<T> {
        /// Issue a new penalty
        fn issue_penalty(
            who: &T::AccountId,
            penalty_type: PenaltyType,
            severity: Severity,
            duration: Option<BlockNumberFor<T>>,
            reporter: T::AccountId,
        ) -> Result<T::PenaltyId, DispatchError> {
            let penalty_id = NextPenaltyId::<T>::mutate(|id| {
                let current = *id;
                *id = T::PenaltyId::from(Into::<u64>::into(*id) + 1);
                current
            });

            let current_block = frame_system::Pallet::<T>::block_number();
            let expires_at = duration.map(|d| current_block + d);

            let penalty = PenaltyRecord {
                id: penalty_id,
                penalty_type: penalty_type.clone(),
                severity: severity.clone(),
                issued_at: current_block,
                expires_at,
                reporter: reporter.clone(),
            };

            // Add to active penalties
            ActivePenalties::<T>::try_mutate(who, |penalties| {
                penalties.try_push(penalty.clone())
                    .map_err(|_| Error::<T>::TooManyPenalties)
            })?;

            // Store penalty details
            PenaltyDetails::<T>::insert(penalty_id, (who.clone(), penalty));

            Self::deposit_event(Event::PenaltyIssued {
                penalty_id,
                who: who.clone(),
                penalty_type,
                severity,
                reporter,
            });

            Ok(penalty_id)
        }

        /// Remove a penalty
        fn remove_penalty(who: &T::AccountId, penalty_id: T::PenaltyId) -> DispatchResult {
            // Remove from active penalties
            ActivePenalties::<T>::mutate(who, |penalties| {
                penalties.retain(|p| p.id != penalty_id);
            });

            // Remove details
            PenaltyDetails::<T>::remove(penalty_id);

            Ok(())
        }

        /// Process expired penalties
        fn process_expired_penalties(now: BlockNumberFor<T>) -> Weight {
            let mut reads = 0u64;
            let mut writes = 0u64;

            // Iterate through all accounts with penalties
            for (account, mut penalties) in ActivePenalties::<T>::iter() {
                reads = reads.saturating_add(1);
                
                let expired: Vec<T::PenaltyId> = penalties
                    .iter()
                    .filter(|p| {
                        if let Some(expires_at) = p.expires_at {
                            expires_at <= now
                        } else {
                            false
                        }
                    })
                    .map(|p| p.id)
                    .collect();

                if !expired.is_empty() {
                    // Remove expired penalties
                    penalties.retain(|p| !expired.contains(&p.id));
                    
                    if penalties.is_empty() {
                        ActivePenalties::<T>::remove(&account);
                    } else {
                        ActivePenalties::<T>::insert(&account, penalties);
                    }
                    writes = writes.saturating_add(1);

                    // Clean up details and emit events
                    for penalty_id in expired {
                        PenaltyDetails::<T>::remove(penalty_id);
                        writes = writes.saturating_add(1);
                        
                        Self::deposit_event(Event::PenaltyExpired {
                            penalty_id,
                            who: account.clone(),
                        });
                    }
                }
            }

            T::DbWeight::get().reads_writes(reads, writes)
        }

        /// Calculate restriction info for an account
        fn calculate_restriction_info(who: &T::AccountId) -> RestrictionInfo {
            let penalties = ActivePenalties::<T>::get(who);
            
            if penalties.is_empty() {
                return RestrictionInfo::default();
            }

            let mut is_blocked = false;
            let mut max_reduction = 0u8;
            let mut max_severity = None;

            for penalty in penalties.iter() {
                match penalty.severity {
                    Severity::Critical => {
                        is_blocked = true;
                        max_severity = Some(Severity::Critical);
                    },
                    Severity::High => {
                        max_reduction = max_reduction.max(75);
                        if max_severity != Some(Severity::Critical) {
                            max_severity = Some(Severity::High);
                        }
                    },
                    Severity::Medium => {
                        max_reduction = max_reduction.max(50);
                        if max_severity.is_none() || max_severity == Some(Severity::Low) {
                            max_severity = Some(Severity::Medium);
                        }
                    },
                    Severity::Low => {
                        max_reduction = max_reduction.max(25);
                        if max_severity.is_none() {
                            max_severity = Some(Severity::Low);
                        }
                    },
                }
            }

            RestrictionInfo {
                is_blocked,
                reduction_percent: if is_blocked { 100 } else { max_reduction },
                active_penalty_count: penalties.len() as u32,
                max_severity,
            }
        }
    }

    // Implement PenaltyReporter for the pallet itself (for manual reporting)
    impl<T: Config> PenaltyReporter<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
        type PenaltyId = T::PenaltyId;

        fn report_penalty(
            who: &T::AccountId,
            penalty_type: PenaltyType,
            severity: Severity,
            duration: Option<BlockNumberFor<T>>,
        ) -> Result<Self::PenaltyId, DispatchError> {
            // For direct calls, use a dummy reporter account
            let reporter = who.clone();
            Self::issue_penalty(who, penalty_type, severity, duration, reporter)
        }

        fn revoke_penalty(penalty_id: Self::PenaltyId) -> DispatchResult {
            let (account, _) = PenaltyDetails::<T>::get(penalty_id)
                .ok_or(Error::<T>::PenaltyNotFound)?;
            
            Self::remove_penalty(&account, penalty_id)?;
            
            Self::deposit_event(Event::PenaltyRevoked {
                penalty_id,
                who: account,
            });
            
            Ok(())
        }
    }

    // Implement RewardRestriction
    impl<T: Config> RewardRestriction<T::AccountId> for Pallet<T> {
        fn has_restriction(who: &T::AccountId) -> bool {
            !ActivePenalties::<T>::get(who).is_empty()
        }

        fn get_restriction(who: &T::AccountId) -> RestrictionInfo {
            Self::calculate_restriction_info(who)
        }
    }
}