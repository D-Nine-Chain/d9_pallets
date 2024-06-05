use super::*;
use crate as pallet_council_vote;
use frame_support::PalletId;
use sp_runtime::{BuildStorage, MultiSignature};
use sp_staking::{EraIndex, SessionIndex};
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;
pub type BlockNumber = u32;
pub type Signature = MultiSignature;
// pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type AccountId = u64;
pub type Balance = u128;
pub type Index = u32;
pub type Hash = sp_core::H256;
frame_support::construct_runtime!(
    pub enum TestRuntime
    {
        System: frame_system,
        Balances: pallet_balances,
        CouncilLock: pallet_d9_council_lock,
        Session: pallet_session,
        Voting: pallet_d9_node_voting,
        Rewards:pallet_d9_node_rewards,
    }
);

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type Call = Call;
    type BlockNumber = u64;
    type Hash = sp_core::H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = sp_std::vec::Vec<u8>;
    type Header = sp_runtime::testing::Header;
    type Event = Event;
    type BlockHashCount = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = ();
    type Balance = u64;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ();
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub const CurrencySubUnits:u32 = 12;
    pub const PalletId:PalletId = PalletId(*b"node/rwd");
}
impl pallet_d9_node_rewards::Config for TestRuntime {
    type CurrencySubUnits = CurrencySubUnits;
    type PalletId = PalletId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
}
struct CouncilSessionManager<T: Config>(sp_std::marker::PhantomData<T>);
impl<T: Config> CouncilSessionManager<AccountId> for CouncilSessionManager<TestRuntime> {
    fn start_pending_votes(session_index: SessionIndex) {
        Self::start_pending_votes(session_index);
    }
    fn end_active_votes(session_index: SessionIndex) {
        Self::end_active_votes(session_index);
    }
    fn get_ranked_nodes() -> Option<Vec<T::AccountId>> {
        let ranked_nodes_option = T::CouncilSessionManager::get_ranked_nodes();
        if ranked_nodes_option.is_none() {
            return None;
        }
        Some(ranked_nodes_option.unwrap())
    }
    fn current_session_index() -> SessionIndex {
        T::CouncilSessionManager::current_session_index()
    }
}
parameter_types! {
    pub const CurrencySubUnits:u32 = 12;
    pub const MaxCandidates:u32 = 288;
    pub const MaxValidatorNodes = 27;
}
impl pallet_d9_node_voting::Config for TestRuntime {
    type CurrencySubUnits = CurrencySubUnits;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type MaxCandidates = MaxCandidates;
    type MaxValidatorNodes = MaxValidatorNodes;
    type NodeRewardManager = Rewards;
}

struct RankingProvider<T: Config>(sp_std::marker::PhantomData<T>);
//TODO - Implement the RankingProvider trait for the RankingProvider struct
impl CouncilLock::RankingProvider<AccountId> for RankingProvider<TestRuntime> {
    fn get_ranked_nodes() -> Option<Vec<AccountId>> {
        Voting::get_sorted_candidates()
    }

    fn current_session_index() -> SessionIndex {
        Session::current_index()
    }
}
parameter_types! {
    pub const LockIdentitfier:[u8;8] = *b"council/";
    pub PalletId:PalletId = PalletId(*b"council/");
    pub const VotingCouncilSize:u32 = 27;
    pub const MinNominatorRank:u32 = 188;
    pub const AssentingVotesThreshold:u32 = 19;
    pub const DisssentingVotesThreshold:u32 = 10;
    pub const NumberOfSessionsBeforeVote:u32 = 2;
}
impl Config for TestRuntime {
    type LockIdentifier = LockIdentitfier;
    type Currency = Balances;
    type LockableCurrency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type PalletId = PalletId;
    type VotingCouncilSize = VotingCouncilSize;
    type MinNominatorRank = MinNominatorRank;
    type AssentingVotesThreshold = AssentingVotesThreshold;
    type DisssentingVotesThreshold = DisssentingVotesThreshold;
    type NumberOfSessionsBeforeVote = NumberOfSessionsBeforeVote;
    type RankingProvider = RankingProvider<AccountId>;
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<TestRuntime>::default()
        .build_storage()
        .unwrap();

    let balanes_genesis_config: pallet_balances::GenesisConfig<TestRuntime> =
        pallet_balances::GenesisConfig {
            balances: vec![
                // id, owner, is_sufficient, min_balance
                (999, 10_000_000_000_000_000_000_000),
            ],
        };

    config.assimilate_storage(&mut storage).unwrap();

    let mut ext: sp_io::TestExternalities = storage.into();
    // Clear thread local vars for https://github.com/paritytech/substrate/issues/10479.
    ext.execute_with(|| take_hooks());
    ext.execute_with(|| System::set_block_number(1));
    ext
}
