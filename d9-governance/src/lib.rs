#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use sp_std::{vec::Vec, marker::PhantomData};

pub use pallet::*;

// Fee handling imports
use pallet_transaction_payment::OnChargeTransaction;
use frame_support::traits::{Currency, WithdrawReasons, ExistenceRequirement};
use sp_runtime::{
    traits::Dispatchable,
    transaction_validity::{TransactionValidityError, InvalidTransaction},
};
use frame_support::dispatch::{DispatchInfo, PostDispatchInfo, DispatchInfoOf, PostDispatchInfoOf};

// Type aliases for fee handler
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type NegativeImbalanceOf<R> = <<R as pallet::Config>::Currency as Currency<
    <R as frame_system::Config>::AccountId,
>>::NegativeImbalance;

// Traits for voting eligibility
pub trait VotingEligibility<AccountId> {
    /// Check if an account can create proposals
    fn can_propose(who: &AccountId) -> bool;
    
    /// Check if an account can vote
    fn can_vote(who: &AccountId) -> bool;
    
    /// Get the voting weight of an account
    fn get_vote_weight(who: &AccountId) -> Option<u64>;
}

// Authority provider trait for other pallets
pub trait AuthorityProvider<AccountId> {
    /// Check if an account is the governance authority
    fn is_authorized_governance(who: &AccountId) -> bool;
}

// Simpler helper function for other pallets to use
pub fn ensure_root_or_governance<T>(origin: T::RuntimeOrigin) -> DispatchResult
where
    T: frame_system::Config,
    T: HasAuthorityProvider<T::AccountId>,
{
    // First try ensure_root
    if ensure_root(origin.clone()).is_ok() {
        return Ok(());
    }
    
    // Then check if it's from governance
    let who = ensure_signed(origin)?;
    ensure!(
        T::AuthorityProvider::is_authorized_governance(&who),
        DispatchError::BadOrigin
    );
    Ok(())
}

// Trait that pallets must implement to use ensure_root_or_governance
pub trait HasAuthorityProvider<AccountId> {
    type AuthorityProvider: AuthorityProvider<AccountId>;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::{Currency, ReservableCurrency};
    use sp_runtime::traits::AccountIdConversion;
    
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config + pallet_d9_parameters::Config + pallet_d9_balances::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// Currency for reserving funds
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        
        /// Amount to reserve when creating a proposal
        type ProposalBond: Get<BalanceOf<Self>>;
        
        /// Voting period in blocks
        type VotingPeriod: Get<Self::BlockNumber>;
        
        /// Minimum voting turnout (as percentage, 0-100)
        type MinimumTurnout: Get<u8>;
        
        /// Minimum approval percentage for a proposal to pass (0-100)
        type MinimumApproval: Get<u8>;
        
        /// Maximum number of active proposals
        type MaxActiveProposals: Get<u32>;
        
        /// Maximum size for call data
        type MaxCallSize: Get<u32> + Clone + Eq;
        
        /// Maximum parameters per contract call template
        type MaxTemplateParams: Get<u32> + Clone + Eq;
        
        /// Voting eligibility provider
        type VotingEligibility: VotingEligibility<Self::AccountId>;
        
        /// Call to execute proposals
        type ProposalCall: Parameter + frame_support::dispatch::Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;
        
        /// The pallet's ID, used for deriving its sovereign account
        type PalletId: Get<PalletId>;
    }

    /// Contract call templates
    #[pallet::storage]
    #[pallet::getter(fn contract_call_templates)]
    pub type ContractCallTemplates<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32, // Template ID
        ContractCallTemplate<T>,
        OptionQuery
    >;
    
    /// Next template ID
    #[pallet::storage]
    #[pallet::getter(fn next_template_id)]
    pub type NextTemplateId<T: Config> = StorageValue<_, u32, ValueQuery>;
    
    /// Active proposals
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32, // Proposal ID
        Proposal<T>,
        OptionQuery
    >;
    
    /// Next proposal ID
    #[pallet::storage]
    #[pallet::getter(fn next_proposal_id)]
    pub type NextProposalId<T: Config> = StorageValue<_, u32, ValueQuery>;
    
    /// Votes on proposals
    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type ProposalVotes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32, // Proposal ID
        Blake2_128Concat,
        T::AccountId,
        (bool, u64), // (approve, weight)
        OptionQuery
    >;
    
    /// Governance admin
    #[pallet::storage]
    #[pallet::getter(fn governance_admin)]
    pub type GovernanceAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;
    
    /// Contract address to receive transaction fees
    #[pallet::storage]
    #[pallet::getter(fn node_reward_contract_address)]
    pub type NodeRewardContractAddress<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Contract call template registered
        TemplateRegistered {
            template_id: u32,
            name: Vec<u8>,
        },
        /// Proposal created
        ProposalCreated {
            proposal_id: u32,
            proposer: T::AccountId,
        },
        /// Vote cast
        VoteCast {
            proposal_id: u32,
            voter: T::AccountId,
            approve: bool,
            weight: u64,
        },
        /// Proposal executed
        ProposalExecuted {
            proposal_id: u32,
        },
        /// Proposal failed
        ProposalFailed {
            proposal_id: u32,
            reason: DispatchError,
        },
        /// Proposal expired
        ProposalExpired {
            proposal_id: u32,
        },
        /// Admin set
        AdminSet {
            new_admin: T::AccountId,
        },
        /// Node reward contract address set
        NodeRewardContractSet {
            contract: Option<T::AccountId>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Not authorized to create proposals
        NotAuthorizedToPropose,
        /// Not authorized to vote
        NotAuthorizedToVote,
        /// Template not found
        TemplateNotFound,
        /// Proposal not found
        ProposalNotFound,
        /// Invalid parameter count
        InvalidParameterCount,
        /// Parameter out of bounds
        ParameterOutOfBounds,
        /// Already voted
        AlreadyVoted,
        /// Proposal not active
        ProposalNotActive,
        /// Voting period ended
        VotingPeriodEnded,
        /// Too many active proposals
        TooManyActiveProposals,
        /// Invalid template name
        InvalidTemplateName,
        /// Invalid parameter spec
        InvalidParameterSpec,
        /// Call size too large
        CallSizeTooLarge,
        /// Not admin
        NotAdmin,
        /// Insufficient turnout
        InsufficientTurnout,
        /// Insufficient approval
        InsufficientApproval,
        /// Invalid parameter type
        InvalidParameterType,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set the governance admin (root only)
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn set_admin(
            origin: OriginFor<T>,
            new_admin: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            GovernanceAdmin::<T>::put(&new_admin);
            Self::deposit_event(Event::AdminSet { new_admin });
            Ok(())
        }
        
        /// Register a contract call template (admin only)
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 2))]
        pub fn register_contract_call_template(
            origin: OriginFor<T>,
            name: Vec<u8>,
            contract: T::AccountId,
            selector: [u8; 4],
            param_specs: Vec<ParamSpec>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                Some(who) == GovernanceAdmin::<T>::get(),
                Error::<T>::NotAdmin
            );
            
            ensure!(!name.is_empty() && name.len() <= 128, Error::<T>::InvalidTemplateName);
            ensure!(
                param_specs.len() <= T::MaxTemplateParams::get() as usize,
                Error::<T>::InvalidParameterSpec
            );
            
            let bounded_name = BoundedVec::try_from(name.clone())
                .map_err(|_| Error::<T>::InvalidTemplateName)?;
            let bounded_params = BoundedVec::try_from(param_specs)
                .map_err(|_| Error::<T>::InvalidParameterSpec)?;
            
            let template_id = NextTemplateId::<T>::get();
            let template = ContractCallTemplate {
                name: bounded_name,
                contract,
                selector,
                param_specs: bounded_params,
            };
            
            ContractCallTemplates::<T>::insert(template_id, template);
            NextTemplateId::<T>::put(template_id + 1);
            
            Self::deposit_event(Event::TemplateRegistered { template_id, name });
            Ok(())
        }
        
        /// Propose a parameter change
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn propose_parameter_change(
            origin: OriginFor<T>,
            pallet_id: Vec<u8>,
            param_id: Vec<u8>,
            new_value: Vec<u8>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(
                T::VotingEligibility::can_propose(&proposer),
                Error::<T>::NotAuthorizedToPropose
            );
            
            // Reserve proposal bond
            <T as Config>::Currency::reserve(&proposer, T::ProposalBond::get())?;
            
            let proposal_id = Self::create_proposal(
                proposer.clone(),
                ProposalType::ParameterChange {
                    pallet_id: BoundedVec::try_from(pallet_id).map_err(|_| Error::<T>::CallSizeTooLarge)?,
                    param_id: BoundedVec::try_from(param_id).map_err(|_| Error::<T>::CallSizeTooLarge)?,
                    new_value: BoundedVec::try_from(new_value).map_err(|_| Error::<T>::CallSizeTooLarge)?,
                },
            )?;
            
            Self::deposit_event(Event::ProposalCreated { proposal_id, proposer });
            Ok(())
        }
        
        /// Propose a contract call
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(4, 2))]
        pub fn propose_contract_call(
            origin: OriginFor<T>,
            template_id: u32,
            params: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(
                T::VotingEligibility::can_propose(&proposer),
                Error::<T>::NotAuthorizedToPropose
            );
            
            // Validate template exists and params match
            let template = ContractCallTemplates::<T>::get(template_id)
                .ok_or(Error::<T>::TemplateNotFound)?;
            
            ensure!(
                params.len() == template.param_specs.len(),
                Error::<T>::InvalidParameterCount
            );
            
            // Validate parameters against specs
            let mut bounded_params = BoundedVec::default();
            for (i, param) in params.iter().enumerate() {
                let spec = &template.param_specs[i];
                Self::validate_parameter(param, spec)?;
                
                let bounded_param = BoundedVec::try_from(param.clone())
                    .map_err(|_| Error::<T>::CallSizeTooLarge)?;
                bounded_params.try_push(bounded_param)
                    .map_err(|_| Error::<T>::CallSizeTooLarge)?;
            }
            
            // Reserve proposal bond
            <T as Config>::Currency::reserve(&proposer, T::ProposalBond::get())?;
            
            let proposal_id = Self::create_proposal(
                proposer.clone(),
                ProposalType::ContractCall {
                    template_id,
                    params: bounded_params,
                },
            )?;
            
            Self::deposit_event(Event::ProposalCreated { proposal_id, proposer });
            Ok(())
        }
        
        /// Propose a fee multiplier change
        #[pallet::call_index(7)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn propose_fee_multiplier_change(
            origin: OriginFor<T>,
            new_multiplier: u128,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;
            ensure!(
                T::VotingEligibility::can_propose(&proposer),
                Error::<T>::NotAuthorizedToPropose
            );
            
            // Reserve proposal bond
            <T as Config>::Currency::reserve(&proposer, T::ProposalBond::get())?;
            
            let proposal_id = Self::create_proposal(
                proposer.clone(),
                ProposalType::FeeMultiplierChange { new_multiplier },
            )?;
            
            Self::deposit_event(Event::ProposalCreated { proposal_id, proposer });
            Ok(())
        }
        
        /// Vote on a proposal
        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 1))]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: u32,
            approve: bool,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            ensure!(
                T::VotingEligibility::can_vote(&voter),
                Error::<T>::NotAuthorizedToVote
            );
            
            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;
            
            ensure!(
                proposal.status == ProposalStatus::Active,
                Error::<T>::ProposalNotActive
            );
            
            let current_block = <frame_system::Pallet<T>>::block_number();
            ensure!(
                current_block <= proposal.end_block,
                Error::<T>::VotingPeriodEnded
            );
            
            ensure!(
                !ProposalVotes::<T>::contains_key(proposal_id, &voter),
                Error::<T>::AlreadyVoted
            );
            
            let weight = T::VotingEligibility::get_vote_weight(&voter)
                .unwrap_or(1); // Default weight of 1 if not found
            
            if approve {
                proposal.yes_votes = proposal.yes_votes.saturating_add(weight);
            } else {
                proposal.no_votes = proposal.no_votes.saturating_add(weight);
            }
            
            Proposals::<T>::insert(proposal_id, &proposal);
            ProposalVotes::<T>::insert(proposal_id, &voter, (approve, weight));
            
            Self::deposit_event(Event::VoteCast {
                proposal_id,
                voter,
                approve,
                weight,
            });
            
            Ok(())
        }
        
        /// Execute a proposal that has passed
        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn execute_proposal(
            origin: OriginFor<T>,
            proposal_id: u32,
        ) -> DispatchResult {
            ensure_signed(origin)?; // Anyone can execute
            
            let mut proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;
            
            ensure!(
                proposal.status == ProposalStatus::Active,
                Error::<T>::ProposalNotActive
            );
            
            let current_block = <frame_system::Pallet<T>>::block_number();
            
            // Check if voting period has ended
            if current_block > proposal.end_block {
                // Check if proposal passed
                let total_votes = proposal.yes_votes.saturating_add(proposal.no_votes);
                let approval_percentage = if total_votes > 0 {
                    (proposal.yes_votes * 100) / total_votes
                } else {
                    0
                };
                
                // Check minimum turnout and approval
                let turnout_ok = total_votes >= (T::MinimumTurnout::get() as u64);
                let approval_ok = approval_percentage >= (T::MinimumApproval::get() as u64);
                
                if turnout_ok && approval_ok {
                    // Execute the proposal
                    let result = match &proposal.proposal_type {
                        ProposalType::ParameterChange { pallet_id, param_id, new_value } => {
                            Self::execute_parameter_change(
                                pallet_id.to_vec(),
                                param_id.to_vec(),
                                new_value.to_vec(),
                            )
                        },
                        ProposalType::ContractCall { template_id, params } => {
                            Self::execute_contract_call(*template_id, params.clone())
                        },
                        ProposalType::FeeMultiplierChange { new_multiplier } => {
                            Self::execute_fee_multiplier_change(*new_multiplier)
                        },
                    };
                    
                    match result {
                        Ok(_) => {
                            proposal.status = ProposalStatus::Executed;
                            Self::deposit_event(Event::ProposalExecuted { proposal_id });
                        },
                        Err(e) => {
                            proposal.status = ProposalStatus::Failed;
                            Self::deposit_event(Event::ProposalFailed {
                                proposal_id,
                                reason: e,
                            });
                        }
                    }
                } else {
                    proposal.status = ProposalStatus::Failed;
                    let reason = if !turnout_ok {
                        Error::<T>::InsufficientTurnout.into()
                    } else {
                        Error::<T>::InsufficientApproval.into()
                    };
                    Self::deposit_event(Event::ProposalFailed { proposal_id, reason });
                }
                
                // Return bond to proposer
                <T as Config>::Currency::unreserve(&proposal.proposer, T::ProposalBond::get());
                
                Proposals::<T>::insert(proposal_id, proposal);
            } else {
                return Err(Error::<T>::VotingPeriodEnded.into());
            }
            
            Ok(())
        }
        
        /// Set the node reward contract address (admin only)
        #[pallet::call_index(6)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn set_node_reward_contract(
            origin: OriginFor<T>,
            contract: Option<T::AccountId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                Some(who) == GovernanceAdmin::<T>::get(),
                Error::<T>::NotAdmin
            );
            
            NodeRewardContractAddress::<T>::set(contract.clone());
            Self::deposit_event(Event::NodeRewardContractSet { contract });
            Ok(())
        }
    }

    // Helper functions
    impl<T: Config> Pallet<T> {
        fn create_proposal(
            proposer: T::AccountId,
            proposal_type: ProposalType<T>,
        ) -> Result<u32, DispatchError> {
            let active_count = Proposals::<T>::iter()
                .filter(|(_, p)| p.status == ProposalStatus::Active)
                .count() as u32;
            
            ensure!(
                active_count < T::MaxActiveProposals::get(),
                Error::<T>::TooManyActiveProposals
            );
            
            let proposal_id = NextProposalId::<T>::get();
            let current_block = <frame_system::Pallet<T>>::block_number();
            let end_block = current_block + T::VotingPeriod::get();
            
            let proposal = Proposal {
                proposer,
                proposal_type,
                start_block: current_block,
                end_block,
                yes_votes: 0,
                no_votes: 0,
                status: ProposalStatus::Active,
            };
            
            Proposals::<T>::insert(proposal_id, proposal);
            NextProposalId::<T>::put(proposal_id + 1);
            
            Ok(proposal_id)
        }
        
        fn validate_parameter(value: &[u8], spec: &ParamSpec) -> DispatchResult {
            match spec.param_type {
                ParameterType::U8 => {
                    ensure!(value.len() >= 1, Error::<T>::InvalidParameterType);
                    let val = value[0] as u128;
                    Self::check_bounds(val, spec)?;
                },
                ParameterType::U32 => {
                    ensure!(value.len() >= 4, Error::<T>::InvalidParameterType);
                    let array: [u8; 4] = value[..4].try_into()
                        .map_err(|_| Error::<T>::InvalidParameterType)?;
                    let val = u32::from_le_bytes(array) as u128;
                    Self::check_bounds(val, spec)?;
                },
                ParameterType::U64 => {
                    ensure!(value.len() >= 8, Error::<T>::InvalidParameterType);
                    let array: [u8; 8] = value[..8].try_into()
                        .map_err(|_| Error::<T>::InvalidParameterType)?;
                    let val = u64::from_le_bytes(array) as u128;
                    Self::check_bounds(val, spec)?;
                },
                ParameterType::U128 => {
                    ensure!(value.len() >= 16, Error::<T>::InvalidParameterType);
                    let array: [u8; 16] = value[..16].try_into()
                        .map_err(|_| Error::<T>::InvalidParameterType)?;
                    let val = u128::from_le_bytes(array);
                    Self::check_bounds(val, spec)?;
                },
                ParameterType::I32 => {
                    ensure!(value.len() >= 4, Error::<T>::InvalidParameterType);
                },
            }
            Ok(())
        }
        
        fn check_bounds(value: u128, spec: &ParamSpec) -> DispatchResult {
            if let Some(min) = spec.min_value {
                ensure!(value >= min, Error::<T>::ParameterOutOfBounds);
            }
            if let Some(max) = spec.max_value {
                ensure!(value <= max, Error::<T>::ParameterOutOfBounds);
            }
            Ok(())
        }
        
        fn execute_parameter_change(
            pallet_id: Vec<u8>,
            param_id: Vec<u8>,
            new_value: Vec<u8>,
        ) -> DispatchResult {
            // Since governance pallet is authorized, we can directly call the parameters pallet
            pallet_d9_parameters::Pallet::<T>::set_parameter(
                frame_system::RawOrigin::Signed(Self::account_id()).into(),
                pallet_id,
                param_id,
                new_value,
            )?;
            
            Ok(())
        }
        
        fn execute_contract_call(
            template_id: u32,
            params: BoundedVec<BoundedVec<u8, T::MaxCallSize>, T::MaxTemplateParams>,
        ) -> DispatchResult {
            let template = ContractCallTemplates::<T>::get(template_id)
                .ok_or(Error::<T>::TemplateNotFound)?;
            
            // Build call data
            let mut call_data = Vec::new();
            call_data.extend_from_slice(&template.selector);
            
            // Encode parameters
            for param in params {
                call_data.extend_from_slice(&param);
            }
            
            // Execute contract call
            let result = pallet_contracts::Pallet::<T>::bare_call(
                Self::account_id(),
                template.contract,
                0u32.into(), // No value transfer
                Weight::from_parts(50_000_000_000, 800_000),
                None,
                call_data,
                false,
                pallet_contracts::Determinism::Enforced,
            );
            
            result.result.map_err(|e| e.into()).map(|_| ())
        }
        
        fn execute_fee_multiplier_change(
            new_multiplier: u128,
        ) -> DispatchResult {
            // Convert u128 to bytes for storage in parameters pallet
            let multiplier_bytes = new_multiplier.to_le_bytes().to_vec();
            
            // Set the fee multiplier in the parameters pallet
            pallet_d9_parameters::Pallet::<T>::set_parameter(
                frame_system::RawOrigin::Signed(Self::account_id()).into(),
                b"d9-governance".to_vec(),
                b"fee_multiplier".to_vec(),
                multiplier_bytes,
            )?;
            
            Ok(())
        }
        
        /// The account ID of the governance pallet
        pub fn account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
    }
    
    // Implement AuthorityProvider
    impl<T: Config> AuthorityProvider<T::AccountId> for Pallet<T> {
        fn is_authorized_governance(who: &T::AccountId) -> bool {
            who == &Self::account_id()
        }
    }
}

// Type definitions
use pallet_d9_parameters::ParameterType;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ParamSpec {
    pub name: BoundedVec<u8, ConstU32<64>>,
    pub param_type: ParameterType,
    pub min_value: Option<u128>,
    pub max_value: Option<u128>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ContractCallTemplate<T: Config> {
    pub name: BoundedVec<u8, ConstU32<128>>,
    pub contract: T::AccountId,
    pub selector: [u8; 4],
    pub param_specs: BoundedVec<ParamSpec, T::MaxTemplateParams>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub enum ProposalType<T: Config> {
    ParameterChange {
        pallet_id: BoundedVec<u8, T::MaxCallSize>,
        param_id: BoundedVec<u8, T::MaxCallSize>,
        new_value: BoundedVec<u8, T::MaxCallSize>,
    },
    ContractCall {
        template_id: u32,
        params: BoundedVec<BoundedVec<u8, T::MaxCallSize>, T::MaxTemplateParams>,
    },
    FeeMultiplierChange {
        new_multiplier: u128,
    },
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct Proposal<T: Config> {
    pub proposer: T::AccountId,
    pub proposal_type: ProposalType<T>,
    pub start_block: T::BlockNumber,
    pub end_block: T::BlockNumber,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub status: ProposalStatus,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProposalStatus {
    Active,
    Executed,
    Failed,
}

/// Custom transaction fee handler
pub struct FeeHandler<R>(PhantomData<R>);

impl<R> OnChargeTransaction<R> for FeeHandler<R>
where
    R: pallet::Config + pallet_transaction_payment::Config,
    <R as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
    type LiquidityInfo = Option<NegativeImbalanceOf<R>>;
    type Balance = BalanceOf<R>;

    fn withdraw_fee(
        who: &<R as frame_system::Config>::AccountId,
        _call: &<R as frame_system::Config>::RuntimeCall,
        _info: &DispatchInfoOf<<R as frame_system::Config>::RuntimeCall>,
        fee: Self::Balance,
        _tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        // Check if we have a contract address configured
        if let Some(contract_address) = pallet::NodeRewardContractAddress::<R>::get() {
            // Withdraw fee from the user
            match <R as pallet::Config>::Currency::withdraw(
                who,
                fee,
                WithdrawReasons::TRANSACTION_PAYMENT,
                ExistenceRequirement::KeepAlive,
            ) {
                Ok(imbalance) => Ok(Some(imbalance)),
                Err(_) => Err(TransactionValidityError::Invalid(
                    InvalidTransaction::Payment
                )),
            }
        } else {
            // No contract configured, return None to use default behavior
            Ok(None)
        }
    }

    fn correct_and_deposit_fee(
        _who: &<R as frame_system::Config>::AccountId,
        _dispatch_info: &DispatchInfoOf<<R as frame_system::Config>::RuntimeCall>,
        _post_info: &PostDispatchInfoOf<<R as frame_system::Config>::RuntimeCall>,
        _corrected_fee: Self::Balance,
        _tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        if let Some(paid) = already_withdrawn {
            // Get the contract address - we know it exists because we checked in withdraw_fee
            if let Some(contract_address) = pallet::NodeRewardContractAddress::<R>::get() {
                // Send fee to contract
                let _ = <R as pallet::Config>::Currency::resolve_creating(&contract_address, paid);
            }
        }
        Ok(())
    }
}