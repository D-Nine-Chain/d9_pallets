#[cfg(test)]
mod tests {
    use super::super::*; // Import everything from the pallet's parent (lib.rs)
    use crate as d9_node_offence; // Provide a local alias for the crate
    use frame_support::{assert_noop, assert_ok, construct_runtime, parameter_types};
    use frame_system as system;
    use frame_system::RawOrigin;
    use sp_core::H256;
    use sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, Dispatchable, IdentityLookup},
    };

    // --- 1. Configure Each Pallet in the Test Runtime ---

    // System Config
    impl system::Config for TestRuntime {
        type BaseCallFilter = frame_support::traits::Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = ();
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = pallet_d9_balances::AccountData<u64>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        // For testing, we don’t need a custom origin:
        type MaxConsumers = frame_support::traits::ConstU32<16>;
    }
    parameter_types! {
        pub const Period: u64 = 10;
        pub const Offset: u64 = 1;
    }
    type PeriodicSessions = pallet_session::PeriodicSessions<Period, Offset>;

    parameter_types! {
        pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
        pub const MaxKeys: u32 = MAX_VALIDATOR_NODES + 10;
        pub const MaxPeerInHeartbeats: u32 = MAX_VALIDATOR_NODES + 10;
        pub const MaxPeerDataEncodingSize: u32 = 1024;
    }
    impl pallet_im_online::Config for TestRuntime {
        type AuthorityId = pallet_im_online::sr25519::AuthorityId;
        type RuntimeEvent = RuntimeEvent;
        type NextSessionRotation = PeriodicSessions;
        type ValidatorSet = Historical;
        type ReportUnresponsiveness = ();
        type UnsignedPriority = ImOnlineUnsignedPriority;
        type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
        type MaxKeys = MaxKeys;
        type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
        type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
    }
    impl pallet_session::historical::Config for Runtime {
        type FullIdentification = pallet_d9_node_voting::ValidatorVoteStats<TestRuntime>;
        type FullIdentificationOf = pallet_d9_node_voting::ValidatorStatsOf<TestRuntime>;
    }
    parameter_types! {
        pub const DepositPerItem: u64 = 1_000   ;
        pub const DepositPerByte: u64 = 1_000;
        pub const DefaultDepositLimit: u64 = 1_000;
        pub Schedule: pallet_contracts::Schedule<TestRuntime> = Default::default();
       pub const MaxCodeLen: u32 = 500 * 1024;
       pub const MaxDebugBufferLen:u32 = 2 * 1024 * 1024;
       pub const MaxStorageKeyLen:u32 = 128;
    }
    impl pallet_contracts::Config for TestRuntime {
        type Time = Timestamp;
        type Randomness = RandomnessCollectiveFlip;
        type Currency = Balances;
        type RuntimeEvent = RuntimeEvent;
        type RuntimeCall = RuntimeCall;
        type CallFilter = Nothing;
        type WeightPrice = pallet_transaction_payment::Pallet<Self>;
        type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
        type ChainExtension = D9ChainExtension;
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

    parameter_types! {
        pub const CurrencySubUnits: u128 = 1_000_000_000_000;
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
    }

    impl pallet_d9_node_offence::Config for TestRuntime {
        type MaxOffendersPerSession = frame_support::traits::ConstU32<10>;
        type RuntimeEvent = RuntimeEvent;
    }
    // Timestamp Config
    parameter_types! {
        pub const MinimumPeriod: u64 = 1;
    }
    impl timestamp::Config for TestRuntime {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
        type WeightInfo = ();
    }

    parameter_types! {
        pub const ExistentialDeposit: u64 = 1;
    }
    impl pallet_d9_balances::Config for TestRuntime {
        type Balance = u64; // Or u128, if you prefer
        type DustRemoval = ();
        type RuntimeEvent = RuntimeEvent; // This is your event type from the system
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = System; // Tells Balances which pallet/store holds account info
        type WeightInfo = ();
        type MaxLocks = ();
        type MaxReserves = ();
        type ReserveIdentifier = [u8; 8];
        type MaxHolds = ();
        type FreezeIdentifier = ();
        type HoldIdentifier = ();
        type MaxFreezes = ();
    }
    // --- 2. Construct the Test Runtime ---

    construct_runtime!(
        pub enum TestRuntime where
            Block = TestBlock,
            NodeBlock = TestBlock,
            UncheckedExtrinsic = TestUncheckedExtrinsic
        {
            System: system::{Pallet, Call, Config, Storage, Event<T>},
            Balances: pallet_d9_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
            NodeOffence: d9_node_offence::{Pallet, Call, Storage, Event<T>},
            ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>},
            NodeVoting: pallet_d9_node_voting::{Pallet, Call, Storage, Event<T>},
            NodeRewards: pallet_d9_node_rewards::{Pallet, Call, Storage, Event<T>},
            Timestamp: timestamp::{Pallet, Call, Storage, Inherent},
            Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        }
    );

    // We also define the “Block” and “UncheckedExtrinsic” used above
    pub type TestBlock = frame_system::mocking::MockBlock<TestRuntime>;
    pub type TestUncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;

    // This is how we can build a signed extrinsic in tests:
    // pub type Extrinsic = TestXt<RuntimeCall, u64>;

    // --- 3. Test Externalities Setup (Genesis State, etc.) ---
    // Utility function to create an environment for testing
    pub fn new_test_ext() -> sp_io::TestExternalities {
        // Any initial storage can be placed in the `system` module here
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();
        // Give some initial balances
        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: vec![
                (1, 1_000_000), // account #1 has 1,000,000 units
                (2, 500_000),   // account #2 has 500,000 units
                (3, 10_000),    // ...
            ],
        }
        .assimilate_storage(&mut storage)
        .unwrap();
        // Extend with your pallet's config if necessary
        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
}
