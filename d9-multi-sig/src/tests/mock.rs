use super::super::*; // Import everything from the pallet's parent (lib.rs)
use super::*; // Import everything from the pallet's parent (lib.rs)
use crate as d9_multi_sig; // Provide a local alias for the crate
                           // use frame_support::{assert_noop, assert_ok, construct_runtime, parameter_types};
use frame_system as system;
use frame_system::RawOrigin;
use pallet_timestamp as timestamp;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::boxed::Box;
use sp_std::vec::Vec;
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
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    // For testing, we don’t need a custom origin:
    type MaxConsumers = frame_support::traits::ConstU32<16>;
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

// Our Pallet’s Config
parameter_types! {
    pub const MaxSignatories: u32 = 5;
    pub const MaxPendingCalls: u32 = 10;
    pub const MaxMultiSigsPerAccountId: u32 = 3;
    pub const MaxCallSize: u32 = 100; // maximum size in bytes for a call
}

// The multi-sig pallet's extrinsics are part of `RuntimeCall` once constructed.
impl d9_multi_sig::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type MaxSignatories = MaxSignatories;
    type MaxPendingCalls = MaxPendingCalls;
    type MaxMultiSigsPerAccountId = MaxMultiSigsPerAccountId;
    type RuntimeCall = RuntimeCall; // from construct_runtime
    type MaxCallSize = MaxCallSize;
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Config for TestRuntime {
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
        Timestamp: timestamp::{Pallet, Call, Storage, Inherent},
        D9MultiSig: d9_multi_sig::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
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
/// Helper function to create a new MSA with `[1,2,3]` signatories (unless you want to vary).
pub fn setup_basic_msa() -> (u64, MultiSignatureAccount<TestRuntime>) {
    // We'll pick user #1 as the "creator" of the MSA
    let origin1 = RawOrigin::Signed(1);
    // Create an MSA with signatories [1,2,3], min approvals = 2
    assert_ok!(D9MultiSig::create_multi_sig_account(
        origin1.into(),
        vec![1, 2, 3],
        None,
        2
    ));
    // Grab the newly created MSA address
    MultiSignatureAccounts::<TestRuntime>::iter()
        .next()
        .unwrap()
}
