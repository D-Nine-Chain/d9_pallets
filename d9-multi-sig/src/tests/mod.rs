// pallets/d9-multi-sig/src/tests/mod.rs

#[cfg(test)]
mod tests {
    use super::super::*; // Import everything from the pallet's parent (lib.rs)
    use crate as d9_multi_sig; // Provide a local alias for the crate
    use frame_support::{assert_noop, assert_ok, construct_runtime, parameter_types};
    use frame_system as system;
    use frame_system::RawOrigin;
    use pallet_timestamp as timestamp;
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

    // --- 4. Example Unit Tests ---
    #[test]
    fn create_multi_sig_account_works() {
        new_test_ext().execute_with(|| {
            // 1) Arrange: set up signatories & call extrinsic
            let origin = RawOrigin::Signed(1);
            let signatories = vec![1, 2, 3];
            let authors = Some(vec![1]); // 1 is definitely in signatories
            let min_approvals = 2;

            // 2) Act: call the extrinsic
            let result = D9MultiSig::create_multi_sig_account(
                origin.into(),
                signatories,
                authors,
                min_approvals,
            );

            // 3) Assert
            assert_ok!(result);

            // We can check that storage was updated, for example:
            // - The newly created MSA address is stable but let’s see if the pallet
            //   constructs the address from signatories.
            // For this mock, you might want to see if MultiSignatureAccounts<T> has something stored.
            // We'll do a partial check to confirm there's at least one MSA
            let maybe_msa = MultiSignatureAccounts::<TestRuntime>::iter().next();
            assert!(maybe_msa.is_some(), "Expected at least one MSA stored");
        });
    }

    #[test]
    fn create_multi_sig_account_fails_for_too_few_signers() {
        new_test_ext().execute_with(|| {
            // signatories must have cardinality >= 2
            let origin = RawOrigin::Signed(1);
            let signatories = vec![1]; // only one signatory
            let min_approvals = 1;

            let result = D9MultiSig::create_multi_sig_account(
                origin.into(),
                signatories,
                None,
                min_approvals,
            );

            // Should fail with SignatoriesListTooShort
            assert_noop!(result, Error::<TestRuntime>::SignatoriesTooShort);
        });
    }

    #[test]
    fn create_msa_fails_without_caller_in_signatories() {
        new_test_ext().execute_with(|| {
            // signatories must have cardinality >= 2
            let origin = RawOrigin::Signed(1);
            let signatories = vec![2, 3, 4]; // only one signatory
            let min_approvals = 1;

            let result = D9MultiSig::create_multi_sig_account(
                origin.into(),
                signatories,
                None,
                min_approvals,
            );

            // Should fail with SignatoriesListTooShort
            assert_noop!(result, Error::<TestRuntime>::CallerNotSignatory);
        });
    }

    #[test]
    fn create_msa_fails_with_duplicates() {
        new_test_ext().execute_with(|| {
            // signatories must have cardinality >= 2
            let origin = RawOrigin::Signed(1);
            let signatories = vec![1, 2, 2]; // only one signatory
            let min_approvals = 1;

            let result = D9MultiSig::create_multi_sig_account(
                origin.into(),
                signatories,
                None,
                min_approvals,
            );

            // Should fail with SignatoriesListTooShort
            assert_noop!(result, Error::<TestRuntime>::DuplicatesInList);
        });
    }

    #[test]
    fn author_a_call_works() {
        new_test_ext().execute_with(|| {
            // 1) Setup a multi-sig with signatories = [1,2,3]
            //    Because we need a multi-sig account to add a call to it.
            let origin = RawOrigin::Signed(1);
            let _ = D9MultiSig::create_multi_sig_account(
                origin.clone().into(),
                vec![1, 2, 3],
                None,
                2, // min_approvals
            );
            // Retrieve the newly created MSA address
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Prepare a "dummy" call
            //    For example, a timestamp call with no arguments.
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 9999 }));

            // 3) Act
            let result =
                D9MultiSig::author_a_call(origin.into(), msa_address.clone(), call.clone());

            // 4) Assert
            assert_ok!(result);
            // Check that the pending call is indeed stored
            let updated_msa = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            assert_eq!(updated_msa.pending_calls.len(), 1);

            let pending_call = &updated_msa.pending_calls[0];
            assert_eq!(
                pending_call.approvals.len(),
                1,
                "Should have 1 approval (from author)"
            );
        });
    }
    #[test]
    fn author_a_call_fails_if_not_author() {
        new_test_ext().execute_with(|| {
            // 1) Setup a multi-sig with signatories = [1,2,3]
            //    Because we need a multi-sig account to add a call to it.
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            let _ = D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                Some(vec![1]), // only 1 is an explicit author
                2,             // min_approvals
            );
            // Retrieve the newly created MSA address
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Prepare a "dummy" call
            //    For example, a timestamp call with no arguments.
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 9999 }));

            // 3) Act
            let result =
                D9MultiSig::author_a_call(origin2.into(), msa_address.clone(), call.clone());

            // 4) Assert
            assert_noop!(result, Error::<TestRuntime>::AccountNotAuthor);
        });
    }

    #[test]
    fn balances_transfer_works() {
        new_test_ext().execute_with(|| {
            // 1) Check initial balances
            assert_eq!(Balances::free_balance(&1), 1_000_000);
            assert_eq!(Balances::free_balance(&2), 500_000);

            // 2) Transfer 100_000 units from #1 to #2
            let transfer_call = pallet_balances::Call::<TestRuntime>::transfer {
                dest: 2,
                value: 100_000,
            };
            let top_level_call = RuntimeCall::Balances(transfer_call);

            // 3) Now .dispatch(...) is available
            assert_ok!(top_level_call.dispatch(RawOrigin::Signed(1).into()));
            // Dispatch as if #1 signed it

            // 3) Check final balances
            assert_eq!(Balances::free_balance(&1), 900_000);
            assert_eq!(Balances::free_balance(&2), 600_000);
        });
    }
    #[test]
    fn add_approval_works_and_triggers_execute_call() {
        new_test_ext().execute_with(|| {
            // Create multi-sig
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            let _ = D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2,
            );
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();
            let msa_vec = UserMultiSigAccounts::<TestRuntime>::get(1).unwrap();
            assert!(msa_vec.contains(&msa_address));
            // send tokens to the multi-sig account
            let transfer_call = pallet_balances::Call::<TestRuntime>::transfer {
                dest: msa_address.clone(),
                value: 100_000,
            };
            let runtime_call = RuntimeCall::Balances(transfer_call);
            assert_ok!(runtime_call.dispatch(RawOrigin::Signed(1).into()));

            // Author a call for the multi-sig account to execute
            let new_msa_runtime_call =
                RuntimeCall::Balances(pallet_balances::Call::<TestRuntime>::transfer {
                    dest: 3,
                    value: 20_000,
                });
            let _ = D9MultiSig::author_a_call(
                origin1.into(),
                msa_address.clone(),
                Box::new(new_msa_runtime_call),
            );

            // Check pending calls
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            assert_eq!(msa_data.pending_calls.len(), 1);
            let call_id = msa_data.pending_calls[0].id;

            // Now have user 2 add an approval. The min_approvals=2, so it should execute immediately.
            let result = D9MultiSig::add_approval(origin2.into(), msa_address.clone(), call_id);
            assert_ok!(result);

            // After execution, that call should be removed from the pending_calls
            let msa_data_post_execution =
                MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            assert_eq!(msa_data_post_execution.pending_calls.len(), 0);
        });
    }

    // More tests for remove_approval, adjust_min_approvals, remove_call, etc. can follow
    #[test]
    fn remove_approval_works() {
        new_test_ext().execute_with(|| {
            // 1) Setup a multi-sig with signatories [1,2,3], min approvals=2
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                3
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Author a call from account 1
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 1000 }));
            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                call
            ));
            let msa_data_after_call =
                MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            let call_id = msa_data_after_call.pending_calls[0].id;
            assert_ok!(D9MultiSig::add_approval(
                origin2.clone().into(),
                msa_address,
                call_id
            ));
            let msa_data_after_approval =
                MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            assert_eq!(
                msa_data_after_approval.pending_calls[0].approvals.len(),
                2,
                "should be two approvals"
            );
            // 4) Remove that approval from account 2
            assert_ok!(D9MultiSig::remove_approval(
                origin2.into(),
                msa_address,
                call_id
            ));
            let msa_data_after_removal =
                MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            // 5) Inspect pending call approvals. The call should still be pending,
            //    but with fewer approvals now.
            assert_eq!(
                msa_data_after_removal.pending_calls.len(),
                1,
                "Call should remain in pending_calls"
            );
            assert_eq!(
                msa_data_after_removal.pending_calls[0].approvals.len(),
                1,
                "Account #2's approval should be removed"
            );
        });
    }

    #[test]
    fn remove_approval_fails_if_not_signatory() {
        new_test_ext().execute_with(|| {
            // Setup a multi-sig with signatories [1,2,3]
            let origin1 = RawOrigin::Signed(1);
            let origin4 = RawOrigin::Signed(4); // not a signatory
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // Author a call
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 1000 }));
            assert_ok!(D9MultiSig::author_a_call(origin1.into(), msa_address, call));

            // Attempt to remove approval from user 4 (not a signatory)
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            let call_id = msa_data.pending_calls[0].id;
            let result = D9MultiSig::remove_approval(origin4.into(), msa_address, call_id);
            assert_noop!(result, Error::<TestRuntime>::AccountNotSignatory);
        });
    }

    #[test]
    fn remove_approval_fails_if_call_not_found() {
        new_test_ext().execute_with(|| {
            // Setup multi-sig [1,2,3]
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // Try removing an approval using a random call_id
            let result = D9MultiSig::remove_approval(
                origin2.into(),
                msa_address,
                [99u8; 32], // This call_id does not exist
            );
            assert_noop!(result, Error::<TestRuntime>::CallNotFound);
        });
    }

    #[test]
    fn adjust_min_approvals_works() {
        new_test_ext().execute_with(|| {
            // 1) Create multi-sig [1,2,3] with min approvals=2
            let origin1 = RawOrigin::Signed(1);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Adjust min approvals from 2 -> 3
            assert_ok!(D9MultiSig::adjust_min_approvals(
                origin1.clone().into(),
                msa_address,
                3
            ));

            // 3) Verify the change
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            assert_eq!(
                msa_data.minimum_signatories, 3,
                "Minimum signatories should now be 3"
            );

            // does not allow min approval of 1
            assert_noop!(
                D9MultiSig::adjust_min_approvals(origin1.into(), msa_address, 1),
                Error::<TestRuntime>::MinApprovalOutOfRange
            );
        });
    }

    #[test]
    fn adjust_min_approvals_fails_if_not_author() {
        new_test_ext().execute_with(|| {
            // 1) Create multi-sig [1,2,3], min approvals=2, no explicit authors => all signatories are authors
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2); // signatory but let's test the scenario where we made only 1 the author
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                Some(vec![1]), // only 1 is an explicit author
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Try adjust min approvals as account 2 (not an author)
            let result = D9MultiSig::adjust_min_approvals(origin2.into(), msa_address, 2);
            assert_noop!(result, Error::<TestRuntime>::AccountNotAuthor);
        });
    }

    #[test]
    fn adjust_min_approvals_fails_for_invalid_range() {
        new_test_ext().execute_with(|| {
            // 1) Create multi-sig [1,2,3] with min approvals=2
            let origin1 = RawOrigin::Signed(1);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2
            ));
            let (msa_address, msa_data) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();
            assert_eq!(msa_data.signatories.len(), 3);

            // 2) Attempt to set new_min_approval to 1 => less than '2..=(3)' range
            let result_too_low =
                D9MultiSig::adjust_min_approvals(origin1.clone().into(), msa_address, 1);
            assert_noop!(result_too_low, Error::<TestRuntime>::MinApprovalOutOfRange);

            // 3) Attempt to set new_min_approval to 4 => more than signatories.len()=3
            let result_too_high = D9MultiSig::adjust_min_approvals(origin1.into(), msa_address, 4);
            assert_noop!(result_too_high, Error::<TestRuntime>::MinApprovalOutOfRange);
        });
    }

    #[test]
    fn remove_call_works() {
        new_test_ext().execute_with(|| {
            // 1) Create multi-sig
            let origin1 = RawOrigin::Signed(1);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Author two calls
            let call1 = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 1234 }));
            let call2 = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 5678 }));
            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                call1
            ));
            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                call2
            ));

            // 3) We now remove the first call
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            let first_call_id = msa_data.pending_calls[0].id;
            assert_ok!(D9MultiSig::remove_call(
                origin1.into(),
                msa_address,
                first_call_id
            ));

            // 4) Check that only one call remains
            let updated_msa = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            assert_eq!(
                updated_msa.pending_calls.len(),
                1,
                "Should have 1 call left"
            );
        });
    }

    #[test]
    fn remove_call_fails_if_not_author() {
        new_test_ext().execute_with(|| {
            // 1) Create a multi-sig, but restrict authors to just [1]
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                Some(vec![1]), // only 1 is allowed to do author-like operations
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Author a call from #1
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 1111 }));
            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                call
            ));

            // 3) #2 tries to remove a call
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            let call_id = msa_data.pending_calls[0].id;
            let result = D9MultiSig::remove_call(origin2.into(), msa_address, call_id);
            assert_noop!(result, Error::<TestRuntime>::AccountNotAuthor);
        });
    }

    #[test]
    fn remove_call_fails_if_not_found() {
        new_test_ext().execute_with(|| {
            // 1) Create multi-sig
            let origin1 = RawOrigin::Signed(1);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                2
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // 2) Attempt to remove a call that doesn't exist
            let fake_call_id = [42u8; 32];
            let result = D9MultiSig::remove_call(origin1.into(), msa_address, fake_call_id);
            assert_noop!(result, Error::<TestRuntime>::CallNotFound);
        });
    }
}
