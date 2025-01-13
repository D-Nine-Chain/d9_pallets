// file: src/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pallet::Error; // if you need to check custom errors
    use frame_support::{assert_noop, assert_ok};
    use sp_runtime::traits::One;

    // Import the mock runtime components we defined:
    use crate::tests::mock::{new_test_ext, NodeOffence, Origin, TestRuntime};

    #[test]
    fn test_submit_candidacy_ok() {
        new_test_ext().execute_with(|| {
            // “Alice” is account ID 1 in this test environment
            let caller = 1u64;
            assert_ok!(NodeOffence::submit_candidacy(Origin::signed(caller)));
        });
    }

    #[test]
    fn test_submit_candidacy_fails_with_unsigned_origin() {
        new_test_ext().execute_with(|| {
            // Attempt with no “signed” origin
            let origin = Origin::none();
            assert_noop!(
                NodeOffence::submit_candidacy(origin),
                frame_support::dispatch::DispatchError::BadOrigin
            );
        });
    }
}
