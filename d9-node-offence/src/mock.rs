// file: src/tests/mock.rs
#![allow(dead_code, unused_imports, unused_variables)]

use crate as d9_node_offence; // rename the crate for clarity
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstBool, ConstU32, ConstU64, Everything, Nothing, OnFinalize, OnInitialize},
    weights::Weight,
};
use frame_system as system;
use pallet_session::{historical as session_historical, PeriodicSessions};
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt, UintAuthorityId},
    traits::{BlakeTwo256, ConvertInto, IdentityLookup},
    Permill,
};
use sp_staking::{
    offence::{OffenceError, ReportOffence},
    SessionIndex,
};
// --------- Types for the mock runtime --------- //

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

// You can use a simpler type alias for convenience.
pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u64;
// --------- Define the mock runtime --------- //
construct_runtime!(
    pub struct TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: system::{Pallet, Call, Storage, Config, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        SessionHistorical: session_historical::{Pallet},
        ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
        NodeVoting: pallet_d9_node_voting::{Pallet, Call, Storage, Event<T>},
        NodeOffence: d9_node_offence::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        Historical: pallet_session::historical::{Pallet},
        Timestamp: pallet_timestamp::{Pallet},
        Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
    }
);

// #[derive(Clone, Eq, PartialEq)]
// pub struct TestRuntime;

// --- System ---
impl system::Config for TestRuntime {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type RuntimeCall = RuntimeCall;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent; // We'll use a single event enum
    type BlockHashCount = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = (); // No balances in this test, or use pallet_balances::AccountData<u64>
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}
impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}
pub struct TestSessionHandler;

impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];
    fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}
    fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
        _changed: bool,
        _validators: &[(AccountId, Ks)],
        _queued_validators: &[(AccountId, Ks)],
    ) {
    }
    fn on_disabled(_validator_index: u32) {}
}

parameter_types! {
pub const Period : BlockNumber = 5;
pub const Offset : BlockNumber = 1;
}
// --- Pallet Session ---
impl pallet_session::Config for TestRuntime {
    type SessionManager = ();
    type Keys = UintAuthorityId;
    type ShouldEndSession = PeriodicSessions<Period, Offset>;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = <Self as system::Config>::AccountId;
    type ValidatorIdOf = IdentityValidator;
    type NextSessionRotation = ();
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

// If you need `pallet_session::historical` (as your logs suggest):
impl session_historical::Config for TestRuntime {
    type FullIdentification = ();
    type FullIdentificationOf = ();
}

// A trivial “convert” to pass a validator id around. In “real” code you’d do more here:
pub struct IdentityValidator;
impl sp_runtime::traits::Convert<AccountId, Option<AccountId>> for IdentityValidator {
    fn convert(a: AccountId) -> Option<AccountId> {
        Some(a)
    }
}
parameter_types! {
    pub static MockAverageSessionLength: Option<u64> = None;
    pub static MockCurrentSessionProgress: Option<Option<Permill>> = None;
}
pub struct TestNextSessionRotation;

impl frame_support::traits::EstimateNextSessionRotation<u64> for TestNextSessionRotation {
    fn average_session_length() -> u64 {
        // take the mock result if any and return it
        let mock = MockAverageSessionLength::mutate(|p| p.take());

        mock.unwrap_or(pallet_session::PeriodicSessions::<Period, Offset>::average_session_length())
    }

    fn estimate_current_session_progress(now: u64) -> (Option<Permill>, Weight) {
        let (estimate, weight) =
            pallet_session::PeriodicSessions::<Period, Offset>::estimate_current_session_progress(
                now,
            );

        // take the mock result if any and return it
        let mock = MockCurrentSessionProgress::mutate(|p| p.take());

        (mock.unwrap_or(estimate), weight)
    }

    fn estimate_next_session_rotation(now: u64) -> (Option<u64>, Weight) {
        pallet_session::PeriodicSessions::<Period, Offset>::estimate_next_session_rotation(now)
    }
}

pub type Extrinsic = TestXt<RuntimeCall, ()>;
type IdentificationTuple = (u64, ());
type Offence = crate::UnresponsivenessOffence<IdentificationTuple>;

parameter_types! {
    pub static Offences: Vec<(Vec<u64>, Offence)> = vec![];
}
parameter_types! {
    pub const DepositPerItem: u64 = 1000   ;
    pub const DepositPerByte: u64 = 1000;
    pub const DefaultDepositLimit: u64 = 1000000000;
    pub Schedule: pallet_contracts::Schedule<TestRuntime> = Default::default();
   pub const MaxCodeLen: u32 =  500 * 1024;
   pub const MaxDebugBufferLen:u32 = 2 * 1024 * 1024;
   pub const MaxStorageKeyLen:u32 = 128;
}
impl pallet_contracts::Config for TestRuntime {
    type Time = Timestamp;
    type Randomness = Randomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Nothing;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    type DepositPerByte = DepositPerByte;
    type DefaultDepositLimit = DefaultDepositLimit;
    type DepositPerItem = DepositPerItem;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = MaxCodeLen;
    type MaxStorageKeyLen = MaxStorageKeyLen;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = MaxDebugBufferLen;
    // #[cfg(not(feature = "runtime-benchmarks"))]
    // type Migrations = ();
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = (NoopMigration<1>, NoopMigration<2>);
}
/// A mock offence report handler.
pub struct OffenceHandler;
impl ReportOffence<u64, IdentificationTuple, Offence> for OffenceHandler {
    fn report_offence(reporters: Vec<u64>, offence: Offence) -> Result<(), OffenceError> {
        Offences::mutate(|l| l.push((reporters, offence)));
        Ok(())
    }

    fn is_known_offence(_offenders: &[IdentificationTuple], _time_slot: &SessionIndex) -> bool {
        false
    }
}
impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}
// --- Pallet ImOnline ---
impl pallet_im_online::Config for TestRuntime {
    type AuthorityId = UintAuthorityId;
    type RuntimeEvent = RuntimeEvent;
    type ValidatorSet = Historical;
    type NextSessionRotation = TestNextSessionRotation;
    type ReportUnresponsiveness = OffenceHandler;
    type UnsignedPriority = ConstU64<{ 1 << 20 }>;
    type WeightInfo = ();
    type MaxKeys = ConstU32<10_000>;
    type MaxPeerInHeartbeats = ConstU32<10_000>;
    type MaxPeerDataEncodingSize = ConstU32<10_000>;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for TestRuntime
where
    RuntimeCall: From<LocalCall>,
{
    type OverarchingCall = RuntimeCall;
    type Extrinsic = Extrinsic;
}

// --- Pallet Transaction Payment ---
parameter_types! {
    // minimal fee multiplier config for example
    pub const TransactionByteFee: u64 = 1;
}

impl pallet_transaction_payment::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    type WeightToFee = frame_support::weights::IdentityFee<u64>;
    type LengthToFee = frame_support::weights::IdentityFee<u64>;
    type OperationalFeeMultiplier = ();
    type FeeMultiplierUpdate = ();
}
pub struct SomeIdentifier(pub [u8; 4]);
parameter_types! {
    pub const MaxLocks: u32 = 50;
    pub const ExistentialDeposit: u64 = 1_000;
    pub const ReserveIdentifier: SomeIdentifier = SomeIdentifier(*b"rsrv");
    pub const FreezeIdentifier: SomeIdentifier = SomeIdentifier(*b"frze");
    pub const MaxHolds: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const MaxFreezes: u32 = 50;
    pub const HoldIdentifier: SomeIdentifier = SomeIdentifier(*b"hold");
}

impl pallet_balances::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<TestRuntime>;
    type Balance = u64;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = ();
    type FreezeIdentifier = ();
    type MaxLocks = MaxLocks;
    type MaxHolds = MaxHolds;
    type MaxReserves = MaxReserves;
    type MaxFreezes = MaxFreezes;
    type HoldIdentifier = ();
}

parameter_types! {
    pub const CurrencySubUnits: u64 = 1_000_000_000_000;
    pub const MaxCandidates: u32 = 10;
    pub const MaxValidatorNodes: u32 = 10;
}
impl pallet_d9_node_voting::Config for TestRuntime {
    type CurrencySubUnits = CurrencySubUnits;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type MaxCandidates = MaxCandidates;
    type MaxValidatorNodes = MaxValidatorNodes;
    type NodeRewardManager = NodeRewards;
    type ReferendumManager = ReferendumManager;
}

// --- Pallet d9-node-offence (the pallet under test!) ---
parameter_types! {
    // Example param for maximum offenders (just to get code working)
    pub const MaxOffendersPerSession: u32 = 10;
}

impl d9_node_offence::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type MaxOffendersPerSession = MaxOffendersPerSession;
}

// --------- Construct the runtime --------- //

// --------- Helper for our tests --------- //

/// Builds a new test externalities environment.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(storage);
    // Optionally: put any initial storage state or setup here
    ext.execute_with(|| {
        // set current block number, etc
        System::set_block_number(1);
    });
    ext
}
