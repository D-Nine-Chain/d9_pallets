#![cfg_attr(not(feature = "std"), no_std)]
use sp_std::prelude::*;
use sp_staking::SessionIndex;
mod structs;
pub use structs::*;
pub use pallet::*;
use frame_support::traits::Currency;

pub type BalanceOf<T> =
    <<T as pallet_contracts::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::{ *, ValueQuery, OptionQuery },
        inherent::Vec,
        BoundedVec,
        weights::Weight,
        Blake2_128Concat,
    };
    //  use sp_std::vec;
    use frame_system::pallet_prelude::*;

    use pallet_session::SessionManager;
    use sp_runtime::Saturating;

    const STORAGE_VERSION: frame_support::traits::StorageVersion = frame_support::traits::StorageVersion::new(
        1
    );
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
    }

    /// defines the voting power of a user
    #[pallet::storage]
    #[pallet::getter(fn vote_tokens)]
    pub type UsersVotingInterests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        VotingInterest,
        OptionQuery
    >;
    /// defines the vote distribution of a user to some candidate
    #[pallet::storage]
    #[pallet::getter(fn vote_delegations)]
    #[pallet::unbounded]
    pub type VoteDelegations<T: Config> = StorageNMap<
        Key = (NMapKey<Blake2_128Concat, T::AccountId>, NMapKey<Blake2_128Concat, T::AccountId>),
        Value = u64,
        QueryKind = ValueQuery
    >;

    /// defines the supporters of a candidate
    #[pallet::storage]
    #[pallet::getter(fn candidate_supporter)]
    #[pallet::unbounded]
    pub type CandidateSupporters<T: Config> = StorageNMap<
        Key = (NMapKey<Blake2_128Concat, T::AccountId>, NMapKey<Blake2_128Concat, T::AccountId>),
        Value = u64,
        QueryKind = ValueQuery
    >;

    /// grand total of votes for a candidate
    ///
    /// this Map can be no larger than MaxCandidates
    #[pallet::storage]
    #[pallet::getter(fn candidate_votes)]
    pub type CandidateAccumulativeVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u64,
        OptionQuery
    >;
    #[pallet::storage]
    #[pallet::getter(fn number_of_candidates)]
    pub type CurrentNumberOfCandidates<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn session_list)]
    pub type SessionCandidateList<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        SessionIndex,
        BoundedVec<T::AccountId, ConstU32<300>>,
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn validator_stats)]
    pub type CurrentValidators<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ValidatorStats<T>,
        OptionQuery
    >;

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
        VoterDidntDelegateToThisCandidate,
        NotActiveValidator,
        AtMaximumNumberOfCandidates,
        BurnAmountMustBeGreaterThan100,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_candidates: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for candidate in self.initial_candidates.iter() {
                CandidateAccumulativeVotes::<T>::insert(candidate.clone(), 1000);
                CurrentNumberOfCandidates::<T>::put(CurrentNumberOfCandidates::<T>::get() + 1);
            }
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { initial_candidates: Default::default() }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn submit_candidacy(origin: OriginFor<T>) -> DispatchResult {
            let validator = ensure_signed(origin)?;
            let current_candidate_count = CurrentNumberOfCandidates::<T>::get();
            let max_candidates = T::MaxCandidates::get();
            if current_candidate_count + 1 > max_candidates {
                return Err(Error::<T>::AtMaximumNumberOfCandidates.into());
            }

            CandidateAccumulativeVotes::<T>::insert(validator.clone(), 0);
            CurrentNumberOfCandidates::<T>::put(current_candidate_count + 1);

            Self::deposit_event(Event::CandidacySubmitted(validator));
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn add_voting_interest(
            origin: OriginFor<T>,
            beneficiary_voter: T::AccountId,
            main_pool: T::AccountId,
            amount_to_burn: BalanceOf<T>,
            burn_contract: T::AccountId
        ) -> DispatchResult {
            let token_burner = ensure_signed(origin)?;
            // Self::call_burn_contract(
            //     token_burner,
            //     beneficiary_voter.clone(),
            //     main_pool,
            //     amount_to_burn,
            //     burn_contract
            // )?;
            let voting_interest_increase = Self::calculate_voting_interests(amount_to_burn);
            let voting_interest = UsersVotingInterests::<T>::mutate(
                beneficiary_voter.clone(),
                |voting_interest_opt| {
                    let voting_interest = voting_interest_opt
                        .clone()
                        .unwrap_or(VotingInterest::default());
                    let new_total = voting_interest.total.saturating_add(voting_interest_increase);
                    VotingInterest {
                        total: new_total,
                        delegated: voting_interest.delegated,
                    }
                }
            );
            UsersVotingInterests::<T>::insert(beneficiary_voter, voting_interest);
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn delegate_votes(
            origin: OriginFor<T>,
            delegations: Vec<ValidatorDelegations<T>>
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

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn remove_candidacy(origin: OriginFor<T>) -> DispatchResult {
            let candidate: T::AccountId = ensure_signed(origin)?;
            let mut support_to_remove: Vec<(T::AccountId, u64)> = Vec::new();
            let mut prefix_iterator = CandidateSupporters::<T>::iter_prefix((candidate.clone(),));
            while let Some((supporter, delegated_votes)) = prefix_iterator.next() {
                support_to_remove.push((supporter, delegated_votes));
            }
            for support in support_to_remove {
                Self::remove_votes_from_candidate(&support.0, &candidate, support.1);
            }
            Self::deposit_event(Event::CandidacyRemoved(candidate));
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn try_remove_votes_from_candidate(
            origin: OriginFor<T>,
            candidate: T::AccountId,
            votes: u64
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            if !Self::is_valid_candidate(&candidate) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }
            let delegated_votes = VoteDelegations::<T>::get((voter.clone(), candidate.clone()));
            if delegated_votes == 0 {
                return Err(Error::<T>::VoterDidntDelegateToThisCandidate.into());
            }
            if votes > delegated_votes {
                return Err(Error::<T>::AttemptingToRemoveMoreVotesThanDelegated.into());
            }

            Self::remove_votes_from_candidate(&voter, &candidate, votes);
            Ok(())
        }
        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn redistribute_votes(
            origin: OriginFor<T>,
            from: T::AccountId,
            to: T::AccountId
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            if !Self::is_valid_candidate(&to) || !Self::is_valid_candidate(&from) {
                return Err(Error::<T>::CandidateDoesNotExist.into());
            }
            let delegated_votes = VoteDelegations::<T>::get((voter.clone(), from.clone()));
            if delegated_votes == 0 {
                return Err(Error::<T>::VoterDidntDelegateToThisCandidate.into());
            }
            Self::remove_votes_from_candidate(&voter, &from, delegated_votes);
            Self::add_votes_to_candidate(&voter, &to, delegated_votes);
            Ok(())
        }
        #[pallet::call_index(6)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn call_contract_test(
            origin: OriginFor<T>,
            token_burner: T::AccountId,
            beneficiary_voter: T::AccountId,
            main_pool: T::AccountId,
            amount: BalanceOf<T>,
            burn_contract: T::AccountId
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::call_burn_contract(
                token_burner,
                beneficiary_voter.clone(),
                main_pool,
                amount,
                burn_contract
            )
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_sorted_candidates() -> Option<Vec<T::AccountId>> {
            let mut candidates = CandidateAccumulativeVotes::<T>
                ::iter()
                .collect::<Vec<(T::AccountId, u64)>>();
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
            burn_contract: T::AccountId
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
            let weight: Weight = Weight::default();
            weight.set_ref_time(500_000_000_000);
            weight.set_proof_size(800_000);

            let contract_call_result = pallet_contracts::Pallet::<T>::bare_call(
                token_burner,
                main_pool,
                amount,
                weight,
                None,
                data_for_contract_call,
                false,
                pallet_contracts::Determinism::Enforced
            ).result;
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
            delegations: &Vec<ValidatorDelegations<T>>
        ) -> Result<(), DispatchError> {
            let available_votes = voting_interest.total.saturating_sub(voting_interest.delegated);
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
            CandidateAccumulativeVotes::<T>::contains_key(candidate.clone())
        }

        fn delegate_votes_to_candidates(
            delegator: &T::AccountId,
            delegations: Vec<ValidatorDelegations<T>>
        ) -> Result<(), DispatchError> {
            for delegation in delegations.iter() {
                let candidate = delegation.candidate.clone();
                let votes = delegation.votes;
                Self::add_votes_to_candidate(delegator, &candidate, votes);
            }
            Ok(())
        }

        fn add_votes_to_candidate(delegator: &T::AccountId, candidate: &T::AccountId, votes: u64) {
            let added_votes = CandidateAccumulativeVotes::<T>::mutate(
                candidate.clone(),
                |candidate_votes_opt| {
                    let candidate_votes = candidate_votes_opt.unwrap();
                    candidate_votes.saturating_add(votes)
                }
            );
            CandidateAccumulativeVotes::<T>::insert(candidate.clone(), added_votes);

            let candidate_support = CandidateSupporters::<T>::mutate(
                (candidate.clone(), delegator.clone()),
                |candidate_supporters| { candidate_supporters.saturating_add(votes) }
            );
            CandidateSupporters::<T>::insert(
                (candidate.clone(), delegator.clone()),
                candidate_support
            );

            let vote_delegation = VoteDelegations::<T>::mutate(
                (delegator.clone(), candidate.clone()),
                |vote_delegations| { vote_delegations.saturating_add(votes) }
            );
            VoteDelegations::<T>::insert((delegator.clone(), candidate.clone()), vote_delegation);

            let user_voting_interests = UsersVotingInterests::<T>::mutate(
                delegator.clone(),
                |voting_interest| {
                    let mut voting_interest = voting_interest.clone().unwrap();
                    voting_interest.delegated = voting_interest.delegated.saturating_add(votes);
                    voting_interest
                }
            );
            UsersVotingInterests::<T>::insert(delegator.clone(), user_voting_interests);
        }

        fn remove_votes_from_candidate(
            delegator: &T::AccountId,
            candidate: &T::AccountId,
            votes: u64
        ) {
            let _ = CandidateAccumulativeVotes::<T>::mutate(
                candidate.clone(),
                |candidate_votes_opt| {
                    let candidate_votes = candidate_votes_opt.unwrap();
                    candidate_votes.saturating_sub(votes)
                }
            );
            let delegated_votes = CandidateSupporters::<T>::get((
                candidate.clone(),
                delegator.clone(),
            ));
            if delegated_votes == votes {
                let _ = CandidateSupporters::<T>::remove((candidate.clone(), delegator.clone()));
                let _ = VoteDelegations::<T>::remove((delegator.clone(), candidate.clone()));
            } else {
                let _ = CandidateSupporters::<T>::mutate(
                    (candidate.clone(), delegator.clone()),
                    |candidate_supporters| { candidate_supporters.saturating_sub(votes) }
                );
                let _ = VoteDelegations::<T>::mutate(
                    (delegator.clone(), candidate.clone()),
                    |vote_delegations| { vote_delegations.saturating_sub(votes) }
                );
            }

            let _ = UsersVotingInterests::<T>::mutate(delegator.clone(), |voting_interest| {
                let mut voting_interest = voting_interest.clone().unwrap();
                voting_interest.delegated = voting_interest.delegated.saturating_sub(votes);
                voting_interest
            });
        }
    }

    impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
        fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
            let sorted_candidates_opt = Self::get_sorted_candidates();
            if sorted_candidates_opt.is_none() {
                return None;
            }
            let mut sorted_candidates = sorted_candidates_opt.unwrap();

            let bounded_candidates: BoundedVec<T::AccountId, ConstU32<300>> = BoundedVec::try_from(
                sorted_candidates.clone()
            ).unwrap();
            SessionCandidateList::<T>::insert(new_index, bounded_candidates);
            sorted_candidates.truncate(T::MaxValidatorNodes::get() as usize);

            Some(sorted_candidates)
        }
        fn start_session(_start_index: SessionIndex) {
            let sorted_candidates_opt = Self::get_sorted_candidates();
            if sorted_candidates_opt.is_none() {
                return;
            }
            let mut sorted_candidates = sorted_candidates_opt.unwrap();

            // if at max candidates, remove the bottom 12
            if CurrentNumberOfCandidates::<T>::get() == T::MaxCandidates::get() {
                if sorted_candidates.len() > (288 as usize) {
                    let to_be_dropped: Vec<T::AccountId> = sorted_candidates
                        .clone()
                        .drain(288..)
                        .collect();
                    for candidate in to_be_dropped {
                        CandidateAccumulativeVotes::<T>::remove(candidate);
                    }
                }
            }

            // store validator stats
            sorted_candidates.truncate(T::MaxValidatorNodes::get() as usize);
            for validator in sorted_candidates.iter() {
                let total_votes_opt = CandidateAccumulativeVotes::<T>::get(validator.clone());
                if total_votes_opt.is_none() {
                    continue;
                }
                let total_votes = total_votes_opt.unwrap();
                let self_votes = total_votes.saturating_sub(
                    CandidateSupporters::<T>::get((validator.clone(), validator.clone()))
                );

                let _ = CurrentValidators::<T>::insert(validator.clone(), ValidatorStats {
                    account_id: validator.clone(),
                    total_votes,
                    self_votes,
                    delegated_votes: total_votes.saturating_sub(self_votes),
                });
            }
        }
        fn end_session(_end_index: SessionIndex) {
            let _ = CurrentValidators::<T>::drain();
        }
    }
}
