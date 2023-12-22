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
        weights::Weight,
        Blake2_128Concat,
    };
    //  use sp_std::vec;
    use frame_system::pallet_prelude::*;
    use pallet_session::SessionManager;
    use serde::de;
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

        type MaxReferralDepth: Get<u32>;

        type SetMaxReferralDepthOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        type MaxCandidates: Get<u64>;
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
    pub type CurrentNumberOfCandidates<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        HexDecodeError,
        EmptyDelegationList,
        DelegationListTooLarge,
        DelegatorHasNoVotingCapacity, // doesnt even have capacity to vote (null votes)
        DelegatorHasNoAvailableVotes, // has already distributed all their votes
        DelegatorHasInsufficientVotes, // has insufficient votes to distribute
        CandidateDoesNotExist,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn submit_candidacy(origin: OriginFor<T>, validator: T::AccountId) -> DispatchResult {
            ensure_signed(origin)?;
            let current_candidate_count = CurrentNumberOfCandidates::<T>::get();
            let max_candidates = T::MaxCandidates::get();
            ensure!(
                current_candidate_count + 1 <= max_candidates,
                "max number of candidates reached"
            );
            CandidateAccumulativeVotes::<T>::insert(validator.clone(), 0);
            CurrentNumberOfCandidates::<T>::put(current_candidate_count + 1);
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
            Self::call_burn_contract(
                token_burner,
                beneficiary_voter.clone(),
                main_pool,
                amount_to_burn,
                burn_contract
            )?;
            let voting_interest_increase = Self::calculate_voting_interests(amount_to_burn);
            Self::update_voting_interests(beneficiary_voter, voting_interest_increase);
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn delegate_votes(
            origin: OriginFor<T>,
            delegations: Vec<ValidatorDelegations<T>>
        ) -> DispatchResult {
            let delegator = ensure_signed(origin)?;
            ensure!(delegations.len() > 0, Error::<T>::EmptyDelegationList);
            ensure!(
                (delegations.len() as u64) <= T::MaxCandidates::get(),
                Error::<T>::DelegationListTooLarge
            );
            let maybe_voting_interest = UsersVotingInterests::<T>::get(delegator.clone());
            ensure!(maybe_voting_interest.is_some(), Error::<T>::DelegatorHasNoVotingCapacity);
            let mut voting_interest = maybe_voting_interest.unwrap();
            Self::validate_delegations(&voting_interest, &delegations)?;
            let _ = Self::distribute_votes(&delegator, &mut voting_interest, delegations);
            Ok(())
        }
    }
    impl<T: Config> Pallet<T> {
        pub fn get_validators() -> Vec<T::AccountId> {
            let mut candidates = CandidateAccumulativeVotes::<T>
                ::iter()
                .collect::<Vec<(T::AccountId, u64)>>();
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            let validators = candidates
                .into_iter()
                .map(|(candidate, _)| candidate)
                .collect::<Vec<T::AccountId>>();
            validators
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
            ensure!(amount > burn_minimum, "amount must be greater than 100");
            let decode_result = hex::decode("0xb1efc17b");
            if decode_result.is_err() {
                return Err(Error::<T>::HexDecodeError.into());
            }
            let mut encoded_selector: Vec<u8> = decode_result.unwrap(); // selector for burn function on burn manager ( future name will be main pool)
            let mut encoded_voter: Vec<u8> = voter.encode();
            let mut encoded_burn_contract: Vec<u8> = burn_contract.encode();
            let mut data_for_contract_call = Vec::new();
            data_for_contract_call.append(&mut encoded_selector);
            data_for_contract_call.append(&mut encoded_voter);
            data_for_contract_call.append(&mut encoded_burn_contract);
            let weight: Weight = Weight::default();
            weight.set_ref_time(50_000_000_000);
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

        fn update_voting_interests(voter: T::AccountId, votes_bought: u64) {
            let mut voting_interest = UsersVotingInterests::<T>
                ::get(voter.clone())
                .unwrap_or(VotingInterest::default());

            voting_interest.total = voting_interest.total.saturating_add(votes_bought);
            UsersVotingInterests::<T>::insert(voter, voting_interest);
        }

        fn validate_delegations(
            voting_interest: &VotingInterest,
            delegations: &Vec<ValidatorDelegations<T>>
        ) -> Result<(), DispatchError> {
            let available_votes = voting_interest.total.saturating_sub(voting_interest.delegated);
            ensure!(available_votes > 0, Error::<T>::DelegatorHasNoAvailableVotes);
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

        fn distribute_votes(
            delegator: &T::AccountId,
            voting_interest: &mut VotingInterest,
            delegations: Vec<ValidatorDelegations<T>>
        ) -> Result<(), DispatchError> {
            let mut votes_distributed = 0;
            for delegation in delegations.iter() {
                let candidate = delegation.candidate.clone();
                let votes = delegation.votes;
                let candidate_votes_maybe = CandidateAccumulativeVotes::<T>::get(candidate.clone());
                ensure!(candidate_votes_maybe.is_some(), Error::<T>::CandidateDoesNotExist);

                let mut candidate_votes = candidate_votes_maybe.unwrap();
                candidate_votes = candidate_votes.saturating_add(votes);
                CandidateAccumulativeVotes::<T>::insert(candidate.clone(), candidate_votes);

                let mut candidate_supporters = CandidateSupporters::<T>::get((
                    candidate.clone(),
                    delegator.clone(),
                ));
                candidate_supporters = candidate_supporters.saturating_add(votes);
                CandidateSupporters::<T>::insert(
                    (candidate.clone(), delegator.clone()),
                    candidate_supporters
                );

                let mut vote_delegations = VoteDelegations::<T>::get((
                    delegator.clone(),
                    candidate.clone(),
                ));

                vote_delegations = vote_delegations.saturating_add(votes);
                VoteDelegations::<T>::insert((delegator.clone(), candidate), vote_delegations);
                votes_distributed = votes_distributed.saturating_add(votes);
            }
            voting_interest.delegated = voting_interest.delegated.saturating_add(votes_distributed);
            UsersVotingInterests::<T>::insert(delegator, voting_interest);
            Ok(())
        }
    }

    //  impl SessionManager<T::AccountId> for Pallet<T> {
    //      fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
    //          None
    //      }
    //      fn start_session(_start_index: SessionIndex) {}
    //      fn end_session(_end_index: SessionIndex) {}
    //      fn new_session_genesis(_now: SessionIndex) -> Option<Vec<T::AccountId>> {
    //          None
    //      }
    //      fn start_session_genesis(_now: SessionIndex) {}
    //      fn end_session_genesis(_now: SessionIndex) {}
    //  }
}
