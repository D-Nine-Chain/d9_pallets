#![cfg_attr(not(feature = "std"), no_std)]
use sp_staking::SessionIndex;
use sp_std::prelude::*;
mod types;
use frame_support::traits::Currency;
pub use pallet::*;
use sp_arithmetic::Perquintill;
pub use types::*;
use pallet_d9_node_primitives::{CandidateManager, NodeMetadataStruct, LivenessEventHandler, VotingEventHandler};

pub type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        inherent::Vec,
        pallet_prelude::{DispatchResult, OptionQuery, ValueQuery, *},
        weights::Weight,
        Blake2_128Concat, BoundedVec,
    };
    //  use sp_std::vec;
    use frame_system::pallet_prelude::{OriginFor, *};

    use pallet_session::SessionManager;
    use sp_runtime::Saturating;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_contracts::Config {
        type CurrencySubUnits: Get<BalanceOf<Self>>;
        type Currency: Currency<Self::AccountId>;
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxCandidates: Get<u32>;
        type MaxValidatorNodes: Get<u32>;
        type NodeRewardManager: NodeRewardManager<Self::AccountId>;
        type ReferendumManager: ReferendumManager;
        type CandidateManager: CandidateManager<Self::AccountId, NodeMetadataStruct>;
        
        /// Handler for voting events
        type VotingEventHandler: VotingEventHandler<Self::AccountId>;
    }

    /// defines the voting power of a user
    #[pallet::storage]
    #[pallet::getter(fn vote_tokens)]
    pub type UsersVotingInterests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, VotingInterest, OptionQuery>;
    /// defines the vote distribution of a user to some candidate
    ///
    /// user -> candidate -> votes
    /// UserToNodeVotesTotals == NodeToUserVotesTotals
    #[pallet::storage]
    #[pallet::getter(fn user_to_node_votes)]
    #[pallet::unbounded]
    pub type UserToNodeVotesTotals<T: Config> = StorageNMap<
        Key = (
            NMapKey<Blake2_128Concat, T::AccountId>,
            NMapKey<Blake2_128Concat, T::AccountId>,
        ),
        Value = u64,
        QueryKind = ValueQuery,
    >;

    /// defines the supporters of a candidate
    ///
    /// candidate -> supporter -> votes
    /// UserToNodeVotesTotals == NodeToUserVotesTotals
    #[pallet::storage]
    #[pallet::getter(fn node_to_user_votes)]
    #[pallet::unbounded]
    pub type NodeToUserVotesTotals<T: Config> = StorageNMap<
        Key = (
            NMapKey<Blake2_128Concat, T::AccountId>,
            NMapKey<Blake2_128Concat, T::AccountId>,
        ),
        Value = u64,
        QueryKind = ValueQuery,
    >;

    /// grand total of votes for a candidate
    ///
    /// this Map can be no larger than MaxCandidates
    #[pallet::storage]
    #[pallet::getter(fn node_votes)]
    pub type NodeAccumulativeVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, OptionQuery>;
    #[pallet::storage]
    #[pallet::getter(fn total_number_of_candidate_nodes)]
    pub type CurrentNumberOfCandidatesNodes<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn session_node_list)]
    pub type SessionNodeList<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        SessionIndex,
        BoundedVec<T::AccountId, ConstU32<300>>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn current_session_index)]
    pub type CurrentSessionIndex<T: Config> = StorageValue<_, SessionIndex, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn current_validator_vote_stats)]
    pub type CurrentValidatorVoteStats<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorVoteStats<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn node_metadata)]
    pub type NodeMetadata<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, NodeMetadataStruct, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pallet_admin)]
    pub type PalletAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CandidacySubmitted(T::AccountId),
        VotesDelegatedBy(T::AccountId),
        CandidacyRemoved(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        HexDecodeError,
        EmptyDelegationList,
        DelegationListTooLarge,
        DelegatorHasNoVotingCapacity, // doesnt even have capacity to vote (null votes)
        DelegatorHasNoAvailableVotes, // has already distributed all their votes
        DelegatorHasInsufficientVotes, // has insufficient votes to distribute
        AttemptingToRemoveMoreVotesThanDelegated,
        CandidateDoesNotExist,
        CandidateAlreadyExists,
        ErrorGettingNodeMetadata,
        VoterDidntDelegateToThisCandidate,
        NotActiveValidator,
        AtMaximumNumberOfCandidates,
        BurnAmountMustBeGreaterThan100,
        SupporterShareOutOfRange,
        CurrentValidatorCanNotChangeSharePercentage,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_candidates: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for candidate in self.initial_candidates.iter() {
                NodeAccumulativeVotes::<T>::insert(candidate.clone(), 1000);
                CurrentNumberOfCandidatesNodes::<T>::put(
                    CurrentNumberOfCandidatesNodes::<T>::get() + 1,
                );
            }
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                initial_candidates: Default::default(),
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(5, 5))]
        pub fn add_voting_interest(
            origin: OriginFor<T>,
            beneficiary_voter: T::AccountId,
            main_pool: T::AccountId,
            amount_to_burn: BalanceOf<T>,
            burn_contract: T::AccountId,
        ) -> DispatchResult {
            let token_burner = ensure_signed(origin)?;
            let weight = Weight::from_parts(50_000_000_000, 800_000);
            Self::call_burn_contract(
                token_burner,
                beneficiary_voter.clone(),
                main_pool,
                amount_to_burn,
                burn_contract,
                weight,
            )?;

            let voting_interest_increase = Self::calculate_voting_interests(amount_to_burn);
            Self::add_voting_interest_internal(beneficiary_voter, voting_interest_increase);
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn delegate_votes(
            origin: OriginFor<T>,
            delegations: Vec<ValidatorDelegations<T>>,
        ) -> DispatchResult {
            let delegator = ensure_signed(origin)?;
            if delegations.len() == 0 {
                return Err(Error::<T>::EmptyDelegationList.into());
            }
            if (delegations.len() as u32) > T::MaxCandidates::get() {
                return Err(Error::<T>::DelegationListTooLarge.into());
            }

            let maybe_voting_interest = UsersVotingInterests::<T>::get(delegator.clone());
            if maybe_voting_interest.is_none() {
                return Err(Error::<T>::DelegatorHasNoVotingCapacity.into());
            }
            let voting_interest = maybe_voting_interest.unwrap();
            Self::validate_delegations(&voting_interest, &delegations)?;
            let _ = Self::delegate_votes_to_candidates(&delegator, delegations);
            Self::deposit_event(Event::VotesDelegatedBy(delegator));
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn remove_candidacy(origin: OriginFor<T>) -> DispatchResult {
            let candidate: T::AccountId = ensure_signed(origin)?;
            let mut support_to_remove: Vec<(T::AccountId, u64)> = Vec::new();
            let mut prefix_iterator = NodeToUserVotesTotals::<T>::iter_prefix((candidate.clone(),));
            while let Some((supporter, delegated_votes)) = prefix_iterator.next() {
                support_to_remove.push((supporter, delegated_votes));
            }
            for support in support_to_remove {
                Self::remove_votes_from_candidate(&support.0, &candidate, support.1);
            }
            let current_candidate_count = CurrentNumberOfCandidatesNodes::<T>::get();
            CurrentNumberOfCandidatesNodes::<T>::put(current_candidate_count - 1);
            NodeMetadata::<T>::remove(candidate.clone());
            Self::deposit_event(Event::CandidacyRemoved(candidate));
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn try_remove_votes_from_candidate(
            origin: OriginFor<T>,
            candidate: T::AccountId,
            votes: u64,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            if !Self::is_valid_candidate(&candidate) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }
            let delegated_votes =
                UserToNodeVotesTotals::<T>::get((voter.clone(), candidate.clone()));
            if delegated_votes == 0 {
                return Err(Error::<T>::VoterDidntDelegateToThisCandidate.into());
            }
            if votes > delegated_votes {
                return Err(Error::<T>::AttemptingToRemoveMoreVotesThanDelegated.into());
            }

            Self::remove_votes_from_candidate(&voter, &candidate, votes);
            Ok(())
        }
        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn redistribute_votes(
            origin: OriginFor<T>,
            from: T::AccountId,
            to: T::AccountId,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            if !Self::is_valid_candidate(&to) || !Self::is_valid_candidate(&from) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }
            let delegated_votes = UserToNodeVotesTotals::<T>::get((voter.clone(), from.clone()));
            if delegated_votes == 0 {
                return Err(Error::<T>::VoterDidntDelegateToThisCandidate.into());
            }
            Self::remove_votes_from_candidate(&voter, &from, delegated_votes);
            Self::add_votes_to_candidate(&voter, &to, delegated_votes);
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn change_candidate_name(
            origin: OriginFor<T>,
            name: BoundedVec<u8, ConstU32<128>>,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            if !Self::is_valid_candidate(&origin) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }
            let node_metadata_option = NodeMetadata::<T>::get(origin.clone());
            if node_metadata_option.is_none() {
                return Err(Error::<T>::ErrorGettingNodeMetadata.into());
            }
            let mut node_metadata = node_metadata_option.unwrap();
            node_metadata.name = name.clone();
            NodeMetadata::<T>::insert(origin.clone(), node_metadata);
            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn change_candidate_supporter_share(
            origin: OriginFor<T>,
            sharing_percent: u8,
        ) -> DispatchResult {
            if sharing_percent > 100 {
                return Err(Error::<T>::SupporterShareOutOfRange.into());
            }

            let node_id = ensure_signed(origin)?;
            if !Self::is_valid_candidate(&node_id) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }

            if CurrentValidatorVoteStats::<T>::contains_key(node_id.clone()) {
                return Err(Error::<T>::CurrentValidatorCanNotChangeSharePercentage.into());
            }
            let node_metadata_option = NodeMetadata::<T>::get(node_id.clone());
            if node_metadata_option.is_none() {
                return Err(Error::<T>::ErrorGettingNodeMetadata.into());
            }
            let mut node_metadata = node_metadata_option.unwrap();
            let current_index = CurrentSessionIndex::<T>::get();
            if node_metadata.index_of_last_percent_change > current_index.saturating_sub(3) {
                return Err(Error::<T>::CurrentValidatorCanNotChangeSharePercentage.into());
            }
            node_metadata.sharing_percent = sharing_percent.clone();
            node_metadata.index_of_last_percent_change = current_index;
            NodeMetadata::<T>::insert(node_id.clone(), node_metadata);
            Ok(())
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_pallet_admin(origin: OriginFor<T>, new_admin: T::AccountId) -> DispatchResult {
            let caller_result = ensure_signed(origin.clone());
            if let Ok(caller) = caller_result {
                let current_admin = PalletAdmin::<T>::get();
                if current_admin.is_some() && current_admin.unwrap() == caller {
                    // Current admin can set new admin
                    PalletAdmin::<T>::put(new_admin);
                    return Ok(());
                }
            }
            // Otherwise need root
            ensure_root(origin)?;
            PalletAdmin::<T>::put(new_admin);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// add voting interest to a user (for vote delegation)
        ///
        /// * `delegator` - the user to add voting interest to
        /// * `votes` - the number of votes to add
        pub fn add_voting_interest_internal(
            delegator: T::AccountId,
            voting_interest_increase: u64,
        ) {
            let voting_interest =
                UsersVotingInterests::<T>::mutate(delegator.clone(), |voting_interest_opt| {
                    let voting_interest = voting_interest_opt
                        .clone()
                        .unwrap_or(VotingInterest::default());
                    let new_total = voting_interest
                        .total
                        .saturating_add(voting_interest_increase);
                    VotingInterest {
                        total: new_total,
                        delegated: voting_interest.delegated,
                    }
                });
            UsersVotingInterests::<T>::insert(delegator, voting_interest);
        }

        pub fn get_sorted_candidates_with_votes() -> Vec<(T::AccountId, u64)> {
            let mut candidates =
                NodeAccumulativeVotes::<T>::iter().collect::<Vec<(T::AccountId, u64)>>();
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            let mut sorted_candidates =
                candidates.into_iter().collect::<Vec<(T::AccountId, u64)>>();

            sorted_candidates.truncate(T::MaxCandidates::get() as usize);
            sorted_candidates
        }

        pub fn get_user_supported_nodes(delegator: T::AccountId) -> Vec<T::AccountId> {
            let delegatees_plus_votes =
                UserToNodeVotesTotals::<T>::iter_prefix((delegator.clone(),))
                    .collect::<Vec<(T::AccountId, u64)>>();

            let delegatees = delegatees_plus_votes
                .into_iter()
                .map(|(delegatee, _)| delegatee)
                .collect();

            delegatees
        }

        pub fn get_node_sharing_percent(node_id: T::AccountId) -> Option<u8> {
            let node_metadata = NodeMetadata::<T>::get(node_id.clone());
            if node_metadata.is_none() {
                return None;
            }
            let candidate_metadata = node_metadata.unwrap();
            Some(candidate_metadata.sharing_percent)
        }

        pub fn get_user_support_ratio(
            delegator: T::AccountId,
            candidate: T::AccountId,
        ) -> Option<Perquintill> {
            let user_to_node_votes =
                UserToNodeVotesTotals::<T>::get((delegator.clone(), candidate.clone()));
            if user_to_node_votes == 0 {
                return None;
            }
            let node_total_votes_option = NodeAccumulativeVotes::<T>::get(candidate.clone());
            if node_total_votes_option.is_none() {
                return None;
            }
            let node_total_votes = node_total_votes_option.unwrap();
            let ratio = Perquintill::from_rational(user_to_node_votes, node_total_votes);
            Some(ratio)
        }

        pub fn get_validator_supporter_share(validator: &T::AccountId) -> u8 {
            let candidate_metadata = NodeMetadata::<T>::get(validator.clone());
            if candidate_metadata.is_none() {
                return 0;
            }
            let candidate_metadata = candidate_metadata.unwrap();
            candidate_metadata.sharing_percent
        }

        pub fn get_sorted_candidates() -> Option<Vec<T::AccountId>> {
            let mut candidates =
                NodeAccumulativeVotes::<T>::iter().collect::<Vec<(T::AccountId, u64)>>();
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            let mut sorted_candidates = candidates
                .into_iter()
                .map(|(candidate, _)| candidate)
                .collect::<Vec<T::AccountId>>();

            sorted_candidates.truncate(T::MaxCandidates::get() as usize);
            match sorted_candidates.len() {
                0 => None,
                _ => Some(sorted_candidates),
            }
        }

        fn call_burn_contract(
            token_burner: T::AccountId,
            voter: T::AccountId,
            main_pool: T::AccountId,
            amount: BalanceOf<T>,
            burn_contract: T::AccountId,
            weight: Weight,
        ) -> Result<(), DispatchError> {
            let decimals: BalanceOf<T> = T::CurrencySubUnits::get();
            let burn_minimum: BalanceOf<T> = <BalanceOf<T>>::from(100u32).saturating_mul(decimals);
            if amount < burn_minimum {
                return Err(Error::<T>::BurnAmountMustBeGreaterThan100.into());
            }
            //0xb1efc17b
            let mut selector: Vec<u8> = [0xb1, 0xef, 0xc1, 0x7b].into();
            let mut encoded_voter: Vec<u8> = voter.encode();
            let mut encoded_burn_contract: Vec<u8> = burn_contract.encode();
            let mut data_for_contract_call = Vec::new();
            data_for_contract_call.append(&mut selector);
            data_for_contract_call.append(&mut encoded_voter);
            data_for_contract_call.append(&mut encoded_burn_contract);

            let contract_call_result = pallet_contracts::Pallet::<T>::bare_call(
                token_burner,
                main_pool,
                amount,
                weight,
                None,
                data_for_contract_call,
                false,
                pallet_contracts::Determinism::Enforced,
            )
            .result;
            if let Err(e) = contract_call_result {
                return Err(e);
            }
            Ok(())
        }

        fn calculate_voting_interests(amount: BalanceOf<T>) -> u64 {
            let sub_units = T::CurrencySubUnits::get();
            let votes_bought = amount / sub_units;
            votes_bought.try_into().unwrap_or(0)
        }

        fn validate_delegations(
            voting_interest: &VotingInterest,
            delegations: &Vec<ValidatorDelegations<T>>,
        ) -> Result<(), DispatchError> {
            let available_votes = voting_interest
                .total
                .saturating_sub(voting_interest.delegated);
            if available_votes == 0 {
                return Err(Error::<T>::DelegatorHasNoAvailableVotes.into());
            }
            let mut votes_to_distribute = 0;
            for delegation in delegations.iter() {
                if !Self::is_valid_candidate(&delegation.candidate) {
                    return Err(Error::<T>::CandidateDoesNotExist.into());
                }
                votes_to_distribute = votes_to_distribute.saturating_add(delegation.votes);
                if votes_to_distribute > available_votes {
                    return Err(Error::<T>::DelegatorHasInsufficientVotes.into());
                }
            }
            Ok(())
        }

        fn is_valid_candidate(candidate: &T::AccountId) -> bool {
            T::CandidateManager::is_candidate(candidate)
        }

        fn delegate_votes_to_candidates(
            delegator: &T::AccountId,
            delegations: Vec<ValidatorDelegations<T>>,
        ) -> Result<(), DispatchError> {
            for delegation in delegations.iter() {
                let candidate = delegation.candidate.clone();
                let votes = delegation.votes;
                Self::add_votes_to_candidate(delegator, &candidate, votes);
            }
            Ok(())
        }

        fn add_votes_to_candidate(delegator: &T::AccountId, candidate: &T::AccountId, votes: u64) {
            let added_votes =
                NodeAccumulativeVotes::<T>::mutate(candidate.clone(), |candidate_votes_opt| {
                    let candidate_votes = candidate_votes_opt.unwrap();
                    candidate_votes.saturating_add(votes)
                });
            NodeAccumulativeVotes::<T>::insert(candidate.clone(), added_votes);

            let candidate_support = NodeToUserVotesTotals::<T>::mutate(
                (candidate.clone(), delegator.clone()),
                |candidate_supporters| candidate_supporters.saturating_add(votes),
            );
            NodeToUserVotesTotals::<T>::insert(
                (candidate.clone(), delegator.clone()),
                candidate_support,
            );

            let vote_delegation = UserToNodeVotesTotals::<T>::mutate(
                (delegator.clone(), candidate.clone()),
                |vote_delegations| vote_delegations.saturating_add(votes),
            );
            UserToNodeVotesTotals::<T>::insert(
                (delegator.clone(), candidate.clone()),
                vote_delegation,
            );

            let user_voting_interests =
                UsersVotingInterests::<T>::mutate(delegator.clone(), |voting_interest| {
                    let mut voting_interest = voting_interest.clone().unwrap();
                    voting_interest.delegated = voting_interest.delegated.saturating_add(votes);
                    voting_interest
                });
            UsersVotingInterests::<T>::insert(delegator.clone(), user_voting_interests);
        }

        fn remove_votes_from_candidate(
            delegator: &T::AccountId,
            candidate: &T::AccountId,
            votes: u64,
        ) {
            let candidate_votes =
                NodeAccumulativeVotes::<T>::mutate(candidate.clone(), |candidate_votes_opt| {
                    let candidate_votes = candidate_votes_opt.unwrap();
                    candidate_votes.saturating_sub(votes)
                });
            NodeAccumulativeVotes::<T>::insert(candidate.clone(), candidate_votes);

            let delegated_votes =
                NodeToUserVotesTotals::<T>::get((candidate.clone(), delegator.clone()));
            if delegated_votes == votes {
                let _ = NodeToUserVotesTotals::<T>::remove((candidate.clone(), delegator.clone()));
                let _ = UserToNodeVotesTotals::<T>::remove((delegator.clone(), candidate.clone()));
            } else {
                let candidate_support = NodeToUserVotesTotals::<T>::mutate(
                    (candidate.clone(), delegator.clone()),
                    |candidate_supporters| candidate_supporters.saturating_sub(votes),
                );
                NodeToUserVotesTotals::<T>::insert(
                    (candidate.clone(), delegator.clone()),
                    candidate_support,
                );
                let vote_delegations = UserToNodeVotesTotals::<T>::mutate(
                    (delegator.clone(), candidate.clone()),
                    |vote_delegations| vote_delegations.saturating_sub(votes),
                );
                UserToNodeVotesTotals::<T>::insert(
                    (delegator.clone(), candidate.clone()),
                    vote_delegations,
                );
            }

            let user_voting_interests =
                UsersVotingInterests::<T>::mutate(delegator.clone(), |voting_interest| {
                    let mut voting_interest = voting_interest.clone().unwrap();
                    voting_interest.delegated = voting_interest.delegated.saturating_sub(votes);
                    voting_interest
                });
            UsersVotingInterests::<T>::insert(delegator.clone(), user_voting_interests);
        }
    }

    impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
        fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
            let sorted_candidates_opt = Self::get_sorted_candidates();
            if sorted_candidates_opt.is_none() {
                return None;
            }
            let mut sorted_candidates = sorted_candidates_opt.unwrap();

            let bounded_candidates: BoundedVec<T::AccountId, ConstU32<300>> =
                BoundedVec::try_from(sorted_candidates.clone()).unwrap();
            SessionNodeList::<T>::insert(new_index, bounded_candidates);
            sorted_candidates.truncate(T::MaxValidatorNodes::get() as usize);

            Some(sorted_candidates)
        }

        fn start_session(start_index: SessionIndex) {
            let _ = CurrentSessionIndex::<T>::put(start_index);
            let _ = T::ReferendumManager::start_pending_votes(start_index);
            let sorted_candidates_opt = Self::get_sorted_candidates();
            if sorted_candidates_opt.is_none() {
                return;
            }
            let mut sorted_candidates = sorted_candidates_opt.unwrap();

            // if at max candidates, remove the bottom 12
            if CurrentNumberOfCandidatesNodes::<T>::get() == T::MaxCandidates::get() {
                if sorted_candidates.len() > (288 as usize) {
                    let to_be_dropped: Vec<T::AccountId> =
                        sorted_candidates.clone().drain(288..).collect();
                    for candidate in to_be_dropped {
                        NodeMetadata::<T>::remove(candidate.clone());
                        let mut support_to_remove: Vec<(T::AccountId, u64)> = Vec::new();
                        let mut prefix_iterator =
                            NodeToUserVotesTotals::<T>::iter_prefix((candidate.clone(),));
                        while let Some((supporter, delegated_votes)) = prefix_iterator.next() {
                            support_to_remove.push((supporter, delegated_votes));
                        }
                        for support in support_to_remove {
                            Self::remove_votes_from_candidate(&support.0, &candidate, support.1);
                        }
                    }
                }
            }

            // store validator stats
            sorted_candidates.truncate(T::MaxValidatorNodes::get() as usize);
            let _ = CurrentValidatorVoteStats::<T>::drain();
            for validator in sorted_candidates.iter() {
                let total_votes_opt = NodeAccumulativeVotes::<T>::get(validator.clone());
                if total_votes_opt.is_none() {
                    continue;
                }
                let total_votes = total_votes_opt.unwrap();
                let self_votes = total_votes.saturating_sub(NodeToUserVotesTotals::<T>::get((
                    validator.clone(),
                    validator.clone(),
                )));

                let _ = CurrentValidatorVoteStats::<T>::insert(
                    validator.clone(),
                    ValidatorVoteStats {
                        account_id: validator.clone(),
                        total_votes,
                        self_votes,
                        delegated_votes: total_votes.saturating_sub(self_votes),
                    },
                );
            }
        }

        fn end_session(end_index: SessionIndex) {
            let _ = CurrentValidatorVoteStats::<T>::drain();
            let sorted_nodes_with_votes = Self::get_sorted_candidates_with_votes();

            let _ = T::NodeRewardManager::update_rewards(end_index, sorted_nodes_with_votes);
            let _ = T::ReferendumManager::end_active_votes(end_index);
        }
    }

    // Implementation to handle candidate management from candidate-registry
    impl<T: Config> Pallet<T> {
        /// Called by candidate-registry when a candidate is added
        pub fn add_candidate_internal(who: &T::AccountId, metadata: NodeMetadataStruct) -> DispatchResult {
            let current_candidate_count = CurrentNumberOfCandidatesNodes::<T>::get();
            let max_candidates = T::MaxCandidates::get();
            if current_candidate_count + 1 > max_candidates {
                return Err(Error::<T>::AtMaximumNumberOfCandidates.into());
            }

            // Initialize with 0 votes
            NodeAccumulativeVotes::<T>::insert(who.clone(), 0);
            CurrentNumberOfCandidatesNodes::<T>::put(current_candidate_count + 1);
            NodeMetadata::<T>::insert(who.clone(), metadata);
            
            // Notify handler
            T::VotingEventHandler::on_candidate_added(who);
            
            Self::deposit_event(Event::CandidacySubmitted(who.clone()));
            Ok(())
        }

        /// Called by candidate-registry when a candidate is removed
        pub fn remove_candidate_internal(who: &T::AccountId) -> DispatchResult {
            if !NodeAccumulativeVotes::<T>::contains_key(who) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }

            // Remove all votes for this candidate
            let mut support_to_remove: Vec<(T::AccountId, u64)> = Vec::new();
            let mut prefix_iterator = NodeToUserVotesTotals::<T>::iter_prefix((who.clone(),));
            while let Some((supporter, delegated_votes)) = prefix_iterator.next() {
                support_to_remove.push((supporter, delegated_votes));
            }
            for support in support_to_remove {
                Self::remove_votes_from_candidate(&support.0, who, support.1);
            }

            // Clean up storage
            NodeAccumulativeVotes::<T>::remove(who);
            NodeMetadata::<T>::remove(who);
            let current_count = CurrentNumberOfCandidatesNodes::<T>::get();
            CurrentNumberOfCandidatesNodes::<T>::put(current_count.saturating_sub(1));

            // Notify handler
            T::VotingEventHandler::on_candidate_removed(who);

            Self::deposit_event(Event::CandidacyRemoved(who.clone()));
            Ok(())
        }
    }

    // Implement liveness event handler
    impl<T: Config> LivenessEventHandler<T::AccountId> for Pallet<T> {
        fn on_promotion_criteria_met(who: &T::AccountId) {
            // Add as candidate with default metadata
            let metadata = NodeMetadataStruct::default();
            
            if let Err(e) = Self::add_candidate_internal(who, metadata) {
                log::error!("Failed to add candidate {:?}: {:?}", who, e);
            }
        }
        
        fn on_stop_tracking(_who: &T::AccountId) {
            // No action needed
        }
    }
}
