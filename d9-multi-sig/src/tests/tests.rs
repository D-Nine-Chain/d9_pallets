use super::*;
pub use frame_support::{assert_noop, assert_ok, construct_runtime, parameter_types};
#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use frame_system::RawOrigin;
    use pallet_timestamp as timestamp;
    use sp_runtime::traits::Dispatchable;
    #[test]
    fn remove_call_approval_fails_if_approval_does_not_exist() {
        new_test_ext().execute_with(|| {
            let (msa_address, _) = setup_basic_msa();

            // Author a dummy call by signatory #1
            let origin1 = RawOrigin::Signed(1);
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 9999 }));
            assert_ok!(D9MultiSig::author_a_call(origin1.into(), msa_address, call));

            // Signatory #2 never approved, so if #2 tries to remove, it fails
            let origin2 = RawOrigin::Signed(2);
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            let call_id = msa_data.pending_calls[0].id;

            // Attempt to remove an approval that doesn't exist
            let result = D9MultiSig::remove_call_approval(origin2.into(), msa_address, call_id);
            assert_noop!(result, Error::<TestRuntime>::ApprovalDoesntExist);
        });
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
            let min_approvals = 2;

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
            let min_approvals = 2;

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
            let transfer_call = pallet_balances::Call::transfer {
                dest: 2,
                value: 100_000,
            };
            let runtime_call = RuntimeCall::Balances(transfer_call);

            // 3) Now .dispatch(...) is available
            assert_ok!(runtime_call.dispatch(RawOrigin::Signed(1).into()));
            // Dispatch as if #1 signed it

            // 3) Check final balances
            assert_eq!(Balances::free_balance(&1), 900_000);
            assert_eq!(Balances::free_balance(&2), 600_000);
        });
    }

    #[test]
    fn add_call_approval_works_and_triggers_execute_call() {
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
            let transfer_call = pallet_balances::Call::transfer {
                dest: msa_address.clone(),
                value: 500_000,
            };
            let runtime_call = RuntimeCall::Balances(transfer_call);
            assert_ok!(runtime_call.dispatch(RawOrigin::Signed(1).into()));

            // Author a call for the multi-sig account to execute
            let new_msa_runtime_call = RuntimeCall::Balances(pallet_balances::Call::transfer {
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
            let result =
                D9MultiSig::add_call_approval(origin2.into(), msa_address.clone(), call_id);
            assert_ok!(result);

            // After execution, that call should be removed from the pending_calls
            let msa_data_post_execution =
                MultiSignatureAccounts::<TestRuntime>::get(&msa_address).unwrap();
            assert_eq!(msa_data_post_execution.pending_calls.len(), 0);
        });
    }

    #[test]
    fn remove_call_approval_works() {
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
            assert_ok!(D9MultiSig::add_call_approval(
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
            assert_ok!(D9MultiSig::remove_call_approval(
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
    fn remove_call_approval_fails_if_not_signatory() {
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
            let result = D9MultiSig::remove_call_approval(origin4.into(), msa_address, call_id);
            assert_noop!(result, Error::<TestRuntime>::AccountNotSignatory);
        });
    }

    #[test]
    fn remove_call_approval_fails_if_call_not_found() {
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
            let result = D9MultiSig::remove_call_approval(
                origin2.into(),
                msa_address,
                [99u8; 32], // This call_id does not exist
            );
            assert_noop!(result, Error::<TestRuntime>::CallNotFound);
        });
    }
    #[test]
    fn proposal_is_removed_when_final_approval_revoked() {
        new_test_ext().execute_with(|| {
            // We'll pick user #1 as the "creator" of the MSA
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            // Create an MSA with signatories [1,2,3], min approvals = 2
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3, 4],
                None,
                3
            ));
            // Grab the newly created MSA address
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // Create proposal to change minimum signatories
            // This creates the proposal and adds origin1's approval
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.clone().into(),
                msa_address,
                4
            ));

            // Verify proposal exists with one approval initially
            let initial_proposal = MinApprovalProposals::<TestRuntime>::get(msa_address);
            assert!(
                initial_proposal.is_some(),
                "Proposal should exist after creation"
            );
            assert_eq!(
                initial_proposal.unwrap().approvals.len(),
                1,
                "Should have one approval from creator"
            );

            // Add second approval
            assert_ok!(D9MultiSig::approve_msa_new_minimum(
                origin2.clone().into(),
                msa_address,
            ));

            // Verify proposal exists with both approvals
            let proposal_after_second = MinApprovalProposals::<TestRuntime>::get(msa_address);
            assert!(
                proposal_after_second.is_some(),
                "Proposal should exist after second approval"
            );
            assert_eq!(
                proposal_after_second.unwrap().approvals.len(),
                2,
                "Should have two approvals"
            );

            // Second signer revokes approval
            assert_ok!(D9MultiSig::revoke_approval_for_msa_new_minimum(
                origin2.into(),
                msa_address
            ));

            // Check state after first revocation
            let proposal_after_first_revoke = MinApprovalProposals::<TestRuntime>::get(msa_address);
            assert!(
                proposal_after_first_revoke.is_some(),
                "Proposal should still exist after one revocation"
            );
            assert_eq!(
                proposal_after_first_revoke.unwrap().approvals.len(),
                1,
                "Should have one approval remaining"
            );

            // First signer (creator) revokes approval
            assert_ok!(D9MultiSig::revoke_approval_for_msa_new_minimum(
                origin1.into(),
                msa_address
            ));

            // Verify proposal is completely removed from storage
            assert!(
                MinApprovalProposals::<TestRuntime>::get(msa_address).is_none(),
                "Proposal should be removed when final approval is revoked"
            );
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
    /// Example test for proposing a new minimum that is *different* from the current one.
    #[test]
    fn proposal_msa_new_minimum_works() {
        new_test_ext().execute_with(|| {
            // 1) Arrange - create a multi-sig
            let (msa_address, _msa_data) = setup_basic_msa();

            // 2) Act: propose a new minimum
            //    Suppose the origin is signatory #1, who is also an author (or signatory).
            let origin1 = RawOrigin::Signed(1);
            let new_min = 3; // must be within [2..=3], because we have 3 signatories
            let result = D9MultiSig::proposal_msa_new_minimum(origin1.into(), msa_address, new_min);

            // 3) Assert
            assert_ok!(result);

            // Also check storage: We should have a new item in MinApprovalProposals
            let proposal_opt = MinApprovalProposals::<TestRuntime>::get(msa_address);
            assert!(proposal_opt.is_some(), "Proposal not found in storage");
            let proposal = proposal_opt.unwrap();
            assert_eq!(proposal.new_minimum, new_min);
            assert_eq!(
                proposal.approvals.len(),
                1,
                "Should have 1 approval from the proposer"
            );
        });
    }

    /// Attempt to propose a new minimum but caller is NOT a valid signatory or author -> fails.
    #[test]
    fn proposal_msa_new_minimum_fails_if_not_author() {
        new_test_ext().execute_with(|| {
            // 1) Setup MSA with signatories = [1,2,3]
            let (msa_address, _) = setup_basic_msa();

            // 2) #4 is not in [1,2,3], so it should fail
            let origin4 = RawOrigin::Signed(4);
            let result = D9MultiSig::proposal_msa_new_minimum(origin4.into(), msa_address, 3);

            // 3) Assert
            assert_noop!(result, Error::<TestRuntime>::AccountNotAuthor);
            assert!(MinApprovalProposals::<TestRuntime>::get(msa_address).is_none());
        });
    }

    /// Approve an existing minimum-approval proposal (happy path).
    #[test]
    fn approve_msa_new_minimum_works() {
        new_test_ext().execute_with(|| {
            // 1) Create MSA with 3 signatories
            let (msa_address, _) = setup_basic_msa();

            // 2) #1 proposes raising min approvals from 2 -> 3
            let origin1 = RawOrigin::Signed(1);
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.into(),
                msa_address,
                3
            ));

            // 3) #2 approves that proposal
            let origin2 = RawOrigin::Signed(2);
            let result = D9MultiSig::approve_msa_new_minimum(origin2.into(), msa_address);
            assert_ok!(result);

            // 4) Because the pass_requirement might be 2 or 3 (depending on your logic),
            //    check if the proposal was *executed* or still pending.
            //    If your logic says "need 2 signatories to raise from 2->3," it should now be applied.
            let updated_msa = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            assert_eq!(
                updated_msa.minimum_signatories, 3,
                "New min should be 3 after approval"
            );

            // Also the proposal should have been removed from storage
            let prop = MinApprovalProposals::<TestRuntime>::get(msa_address);
            assert!(prop.is_none(), "Proposal should be removed if it executed");
        });
    }

    /// Approve fails if the caller is not a signatory or if there is no existing proposal.
    #[test]
    fn approve_msa_new_minimum_fails_not_signatory() {
        new_test_ext().execute_with(|| {
            let (msa_address, _) = setup_basic_msa();

            // #1 proposes a new minimum
            let origin1 = RawOrigin::Signed(1);
            let _ = D9MultiSig::proposal_msa_new_minimum(origin1.into(), msa_address, 3);

            // #4 tries to approve but is not a signatory
            let origin4 = RawOrigin::Signed(4);
            let result = D9MultiSig::approve_msa_new_minimum(origin4.into(), msa_address);
            assert_noop!(result, Error::<TestRuntime>::AccountNotSignatory);
        });
    }

    /// Test revoking an approval that we already added.
    #[test]
    fn revoke_approval_for_msa_new_minimum_works() {
        new_test_ext().execute_with(|| {
            // 1) Setup MSA
            let (msa_address, _) = setup_basic_msa();

            // 2) #1 proposes new min
            let origin1 = RawOrigin::Signed(1);
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.clone().into(),
                msa_address,
                3
            ));

            // 3) #1 then revokes.
            //    That might remove the proposal entirely if #1 was the only approval.
            let result =
                D9MultiSig::revoke_approval_for_msa_new_minimum(origin1.into(), msa_address);
            assert_ok!(result);

            let proposal_opt = MinApprovalProposals::<TestRuntime>::get(msa_address);
            assert!(
                proposal_opt.is_none(),
                "With zero approvals left, the proposal should be removed entirely"
            );
        });
    }

    /// Attempt to revoke an approval that doesn't exist -> fails
    #[test]
    fn revoke_approval_for_msa_new_minimum_fails_if_no_approval() {
        new_test_ext().execute_with(|| {
            // 1) Setup MSA with signatories = [1,2,3]
            let (msa_address, _) = setup_basic_msa();

            // 2) #1 proposes a new minimum, so the proposal exists in storage
            let origin1 = RawOrigin::Signed(1);
            let new_min = 3;
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.into(),
                msa_address,
                new_min
            ));

            // 3) #2 tries to revoke an approval that #2 never gave
            let origin2 = RawOrigin::Signed(2);
            let result =
                D9MultiSig::revoke_approval_for_msa_new_minimum(origin2.into(), msa_address);

            // 4) Assert that it fails with ApprovalDoesntExist
            assert_noop!(result, Error::<TestRuntime>::ApprovalDoesntExist);
        });
    }
    #[test]
    fn new_minimum_executes_pending_calls_if_threshold_met() {
        new_test_ext().execute_with(|| {
            // Setup MSA with 3 signers, requiring 3 approvals
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3],
                None,
                3 // Require all 3 signers
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();

            // Fund the MSA
            let transfer_call = pallet_balances::Call::transfer {
                dest: msa_address,
                value: 100_000,
            };
            assert_ok!(RuntimeCall::Balances(transfer_call).dispatch(origin1.clone().into()));

            // Author a transfer call that needs 3 approvals
            let pending_transfer = RuntimeCall::Balances(pallet_balances::Call::transfer {
                dest: 4,
                value: 50_000,
            });
            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                Box::new(pending_transfer)
            ));

            // Get initial balances
            let initial_msa_balance = Balances::free_balance(msa_address);
            let initial_dest_balance = Balances::free_balance(4);

            // Have 2 signers approve but it won't execute yet (needs 3)
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            let call_id = msa_data.pending_calls[0].id;
            assert_ok!(D9MultiSig::add_call_approval(
                origin2.clone().into(),
                msa_address,
                call_id
            ));

            // Verify call hasn't executed yet
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            assert_eq!(msa_data.pending_calls.len(), 1);
            assert_eq!(Balances::free_balance(msa_address), initial_msa_balance);
            assert_eq!(Balances::free_balance(4), initial_dest_balance);

            // Now propose and approve lowering minimum to 2
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.clone().into(),
                msa_address,
                2
            ));
            assert_ok!(D9MultiSig::approve_msa_new_minimum(
                origin2.into(),
                msa_address
            ));

            // Verify call was executed due to lowered threshold
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            assert_eq!(msa_data.pending_calls.len(), 0);
            assert_eq!(
                Balances::free_balance(msa_address),
                initial_msa_balance - 50_000
            );
            assert_eq!(Balances::free_balance(4), initial_dest_balance + 50_000);
        });
    }

    #[test]
    fn new_minimum_executes_multiple_pending_calls() {
        new_test_ext().execute_with(|| {
            // Setup MSA with 3 signers, requiring 3 approvals
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

            // Fund the MSA
            let transfer_call = pallet_balances::Call::transfer {
                dest: msa_address,
                value: 200_000,
            };
            assert_ok!(RuntimeCall::Balances(transfer_call).dispatch(origin1.clone().into()));

            // Author two transfer calls
            let pending_transfer1 = RuntimeCall::Balances(pallet_balances::Call::transfer {
                dest: 4,
                value: 50_000,
            });
            let pending_transfer2 = RuntimeCall::Balances(pallet_balances::Call::transfer {
                dest: 5,
                value: 30_000,
            });

            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                Box::new(pending_transfer1)
            ));
            assert_ok!(D9MultiSig::author_a_call(
                origin1.clone().into(),
                msa_address,
                Box::new(pending_transfer2)
            ));

            // Get both call IDs and have second signer approve both
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            let call_id1 = msa_data.pending_calls[0].id;
            let call_id2 = msa_data.pending_calls[1].id;

            assert_ok!(D9MultiSig::add_call_approval(
                origin2.clone().into(),
                msa_address,
                call_id1
            ));
            assert_ok!(D9MultiSig::add_call_approval(
                origin2.clone().into(),
                msa_address,
                call_id2
            ));

            // Initial state check
            let initial_msa_balance = Balances::free_balance(msa_address);
            let initial_dest4_balance = Balances::free_balance(4);
            let initial_dest5_balance = Balances::free_balance(5);

            // Lower minimum to 2 and both calls should execute
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.clone().into(),
                msa_address,
                2
            ));
            assert_ok!(D9MultiSig::approve_msa_new_minimum(
                origin2.into(),
                msa_address
            ));

            // Verify both calls executed
            let msa_data = MultiSignatureAccounts::<TestRuntime>::get(msa_address).unwrap();
            assert_eq!(msa_data.pending_calls.len(), 0);
            assert_eq!(
                Balances::free_balance(msa_address),
                initial_msa_balance - 80_000
            );
            assert_eq!(Balances::free_balance(4), initial_dest4_balance + 50_000);
            assert_eq!(Balances::free_balance(5), initial_dest5_balance + 30_000);
        });
    }

    #[test]
    fn proposal_msa_new_minimum_fails_if_proposal_exists() {
        new_test_ext().execute_with(|| {
            // We'll pick user #1 as the "creator" of the MSA
            let origin1 = RawOrigin::Signed(1);
            let origin2 = RawOrigin::Signed(2);
            // Create an MSA with signatories [1,2,3], min approvals = 2
            assert_ok!(D9MultiSig::create_multi_sig_account(
                origin1.clone().into(),
                vec![1, 2, 3, 4],
                None,
                3
            ));
            let (msa_address, _) = MultiSignatureAccounts::<TestRuntime>::iter()
                .next()
                .unwrap();
            // First proposal succeeds
            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.into(),
                msa_address,
                4
            ));

            // Second proposal should fail
            let result = D9MultiSig::proposal_msa_new_minimum(origin2.into(), msa_address, 2);
            assert_noop!(result, Error::<TestRuntime>::ProposalAlreadyPending);
        });
    }

    #[test]
    fn approve_msa_new_minimum_fails_if_same_signer_tries_to_approve() {
        new_test_ext().execute_with(|| {
            let (msa_address, _) = setup_basic_msa();
            let origin1 = RawOrigin::Signed(1);

            assert_ok!(D9MultiSig::proposal_msa_new_minimum(
                origin1.clone().into(),
                msa_address,
                3
            ));

            // Try to approve twice with same signer
            let result = D9MultiSig::approve_msa_new_minimum(origin1.into(), msa_address);
            assert_noop!(result, Error::<TestRuntime>::ApprovalExists);
        });
    }

    #[test]
    fn proposal_msa_new_minimum_fails_if_invalid_minimum() {
        new_test_ext().execute_with(|| {
            let (msa_address, _) = setup_basic_msa();
            let origin1 = RawOrigin::Signed(1);

            // Try to set minimum higher than number of signatories
            let result = D9MultiSig::proposal_msa_new_minimum(
                origin1.clone().into(),
                msa_address,
                4, // Only 3 signatories exist
            );
            assert_noop!(result, Error::<TestRuntime>::MinApprovalOutOfRange);

            // Try to set minimum to 1
            let result = D9MultiSig::proposal_msa_new_minimum(
                origin1.into(),
                msa_address,
                1, // Minimum should be at least 2
            );
            assert_noop!(result, Error::<TestRuntime>::MinApprovalOutOfRange);
        });
    }

    #[test]
    fn author_a_call_fails_if_too_many_pending() {
        new_test_ext().execute_with(|| {
            let (msa_address, _) = setup_basic_msa();
            let origin1 = RawOrigin::Signed(1);

            // Author max number of calls
            for i in 0..10 {
                let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: i }));
                assert_ok!(D9MultiSig::author_a_call(
                    origin1.clone().into(),
                    msa_address,
                    call
                ));
            }

            // One more should fail
            let call = Box::new(RuntimeCall::Timestamp(timestamp::Call::set { now: 101 }));
            let result = D9MultiSig::author_a_call(origin1.into(), msa_address, call);
            assert_noop!(result, Error::<TestRuntime>::CallLimit);
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
