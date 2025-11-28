# Proposal: Vote Lockup Period for Node Delegation

**Issue:** #16
**Author:** D9 Core Team
**Status:** Draft
**Created:** 2024-11-27
**Affects:** `d9-node-voting`

---

## 1. Problem Statement

### 1.1 Vulnerability Description

A vote manipulation vulnerability exists in the council lock (freeze/unfreeze) voting system that allows the same underlying stake to influence a referendum multiple times.

### 1.2 Attack Vector

1. **Node A** (ranked in top 27) casts a vote on a freeze/unfreeze referendum
2. Supporter **withdraws votes** from Node A via `try_remove_votes_from_candidate()` or `redistribute_votes()`
3. Supporter **delegates those votes** to Node B, pushing Node B into top 27
4. **Node B** (now in top 27) casts another vote on the **same referendum**
5. **Result:** The same economic stake has influenced the vote twice

### 1.3 Root Cause

Vote delegation can be withdrawn and redistributed at any time with no restrictions, allowing rapid manipulation of node rankings during active governance votes.

---

## 2. Proposed Solution

### 2.1 Overview

Implement a **vote lockup period** in `d9-node-voting`. When a user delegates votes to a node, those votes are locked for a minimum period (default: 14 days) before they can be withdrawn or redistributed.

### 2.2 Design Principles

1. **Commitment model:** Votes represent a term-based commitment, like an election cycle
2. **Direct prevention:** Votes cannot move during lockup, eliminating manipulation at the source
3. **Predictable behavior:** Users know exactly when their votes unlock
4. **No migration required:** Additive change only

---

## 3. Technical Specification

### 3.1 New Config Constant

Add to `d9-node-voting` Config trait:

```rust
/// Minimum lock period for delegated votes (in blocks)
/// Default: 14 days = 14 * 24 * 60 * 60 / 6 = 201,600 blocks (assuming 6s blocks)
#[pallet::constant]
type VoteLockPeriod: Get<BlockNumberFor<Self>>;
```

### 3.2 New Storage Item

Add to `d9-node-voting/src/lib.rs`:

```rust
/// Tracks when delegated votes can be withdrawn.
/// Key: (voter, candidate) -> Value: block number when lock expires
#[pallet::storage]
#[pallet::getter(fn vote_lock_expiry)]
pub type VoteLockExpiry<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    (T::AccountId, T::AccountId),
    BlockNumberFor<T>,
    OptionQuery
>;
```

### 3.3 Modify `delegate_votes()`

When votes are delegated, set or extend the lock expiry:

```rust
pub fn delegate_votes(
    origin: OriginFor<T>,
    delegations: Vec<ValidatorDelegation<T::AccountId>>,
) -> DispatchResult {
    let voter = ensure_signed(origin)?;
    // ... existing validation ...

    let current_block = <frame_system::Pallet<T>>::block_number();
    let lock_until = current_block + T::VoteLockPeriod::get();

    for delegation in delegations.iter() {
        // ... existing delegation logic ...

        // Set lock expiry for this delegation
        VoteLockExpiry::<T>::insert(
            (voter.clone(), delegation.candidate.clone()),
            lock_until
        );
    }

    // ... rest of existing logic ...
    Ok(())
}
```

### 3.4 Modify `try_remove_votes_from_candidate()`

Check lock before allowing withdrawal:

```rust
pub fn try_remove_votes_from_candidate(
    origin: OriginFor<T>,
    candidate: T::AccountId,
    votes: u64,
) -> DispatchResult {
    let voter = ensure_signed(origin)?;

    // Check if votes are still locked
    Self::ensure_votes_unlocked(&voter, &candidate)?;

    // ... existing logic ...
}
```

### 3.5 Modify `redistribute_votes()`

Check lock before allowing redistribution:

```rust
pub fn redistribute_votes(
    origin: OriginFor<T>,
    from: T::AccountId,
    to: T::AccountId,
) -> DispatchResult {
    let voter = ensure_signed(origin)?;

    // Check if votes are still locked on source candidate
    Self::ensure_votes_unlocked(&voter, &from)?;

    // ... existing logic ...

    // Set new lock on destination candidate
    let current_block = <frame_system::Pallet<T>>::block_number();
    let lock_until = current_block + T::VoteLockPeriod::get();
    VoteLockExpiry::<T>::insert((voter.clone(), to.clone()), lock_until);

    Ok(())
}
```

### 3.6 Helper Function

```rust
impl<T: Config> Pallet<T> {
    /// Ensures votes from voter to candidate are not locked
    fn ensure_votes_unlocked(
        voter: &T::AccountId,
        candidate: &T::AccountId,
    ) -> Result<(), Error<T>> {
        if let Some(lock_expiry) = VoteLockExpiry::<T>::get((voter.clone(), candidate.clone())) {
            let current_block = <frame_system::Pallet<T>>::block_number();
            if current_block < lock_expiry {
                return Err(Error::<T>::VotesStillLocked);
            }
        }
        Ok(())
    }

    /// Returns remaining lock time in blocks (for UI/queries)
    pub fn get_vote_lock_remaining(
        voter: &T::AccountId,
        candidate: &T::AccountId,
    ) -> Option<BlockNumberFor<T>> {
        if let Some(lock_expiry) = VoteLockExpiry::<T>::get((voter.clone(), candidate.clone())) {
            let current_block = <frame_system::Pallet<T>>::block_number();
            if current_block < lock_expiry {
                return Some(lock_expiry - current_block);
            }
        }
        None
    }
}
```

### 3.7 New Error Type

```rust
#[pallet::error]
pub enum Error<T> {
    // ... existing errors ...

    /// Votes are still locked and cannot be withdrawn or redistributed
    VotesStillLocked,
}
```

---

## 4. Behavioral Changes

### 4.1 Before (Current Behavior)

```
User delegates votes -> Can withdraw/redistribute immediately
```

### 4.2 After (Proposed Behavior)

```
User delegates votes -> Votes locked for 14 days -> Can withdraw/redistribute after lock expires
```

---

## 5. Downstream Consequences

### 5.1 Affected: User Experience

| Action | Before | After |
|--------|--------|-------|
| Delegate votes | Immediate, no restrictions | Votes locked for 14 days |
| Withdraw votes | Anytime | Only after lock period expires |
| Redistribute votes | Anytime | Only after lock expires; new lock starts |
| Add more votes to same node | Anytime | Allowed, extends lock period |

### 5.2 Affected: `d9-council-lock`

**No code changes needed.** The vulnerability is prevented at the source - votes cannot be moved to manipulate rankings during a referendum.

### 5.3 NOT Affected: Contracts

All contracts continue to work unchanged. They don't interact with vote locking.

### 5.4 NOT Affected: Reward Distribution

Rewards are based on vote counts at session end. Lock status doesn't affect reward calculation.

### 5.5 NOT Affected: Validator Selection

Validator selection uses current vote counts. Lock status doesn't affect selection.

---

## 6. Migration & Deployment

### 6.1 Storage Migration

**Not required.** This is an additive change:

- New `VoteLockExpiry` storage starts empty
- Existing delegations have no lock (grandfathered in)
- New delegations after upgrade will have locks

### 6.2 Handling Existing Delegations

Existing vote delegations made before the upgrade will NOT have a lock period. Only new delegations (or redistributions) after the upgrade will be subject to the lock.

**Alternative:** If all existing delegations should be locked, a one-time migration can set lock expiry for all existing `UserToNodeVotesTotals` entries.

### 6.3 Deployment Sequence

1. Deploy runtime upgrade with new pallet code
2. New delegations immediately subject to 14-day lock
3. Existing delegations remain unlocked (or migrated if desired)

---

## 7. Configuration

### 7.1 Default Lock Period

```rust
// In runtime configuration
parameter_types! {
    // 14 days assuming 6-second blocks
    // 14 * 24 * 60 * 60 / 6 = 201,600 blocks
    pub const VoteLockPeriod: BlockNumber = 201_600;
}

impl pallet_d9_node_voting::Config for Runtime {
    // ...
    type VoteLockPeriod = VoteLockPeriod;
}
```

### 7.2 Governance Adjustability

The lock period can be changed via runtime upgrade. Consider adding a setter function for admin adjustment if needed.

---

## 8. Testing Requirements

### 8.1 Unit Tests

1. **Lock set on delegation:** Verify `VoteLockExpiry` is set when delegating
2. **Withdrawal blocked:** Verify `try_remove_votes_from_candidate` fails during lock
3. **Redistribution blocked:** Verify `redistribute_votes` fails during lock
4. **Unlock after period:** Verify operations succeed after lock expires
5. **Lock extension:** Verify adding votes extends the lock period

### 8.2 Integration Tests

1. **Attack prevention:** Verify vote recycling attack fails due to lock
2. **Normal operations:** Verify users can delegate, wait, then withdraw normally

### 8.3 Scenario Tests

```
Scenario: Vote recycling attack prevention
  Given User X has 1000 votes delegated to Node A
  And Node A is rank 5
  And a referendum is active
  When User X tries to redistribute votes to Node B
  Then the transaction should fail with VotesStillLocked error
  And Node A should retain all votes
```

---

## 9. Risks & Mitigations

### 9.1 Risk: User Frustration

**Concern:** Users may be frustrated they can't move votes freely.

**Mitigation:**
- Clear UI messaging about lock period before delegation
- Show countdown to unlock in wallet/UI
- 14 days is reasonable for governance commitment

### 9.2 Risk: Emergency Situations

**Concern:** User needs to move votes urgently (e.g., node becomes malicious).

**Mitigation:**
- Consider admin override for emergency unlocks
- 14-day period is not excessively long
- Users should do due diligence before delegating

### 9.3 Risk: Existing Delegations Unprotected

**Concern:** Existing delegations before upgrade have no lock.

**Mitigation:**
- Optional migration to lock existing delegations
- Or accept that old delegations are grandfathered
- New delegations will be locked, eventually all votes will be covered

---

## 10. Implementation Checklist

- [ ] Add `VoteLockPeriod` config constant to `d9-node-voting`
- [ ] Add `VoteLockExpiry` storage map
- [ ] Add `VotesStillLocked` error type
- [ ] Add `ensure_votes_unlocked()` helper function
- [ ] Add `get_vote_lock_remaining()` query function
- [ ] Modify `delegate_votes()` to set lock
- [ ] Modify `try_remove_votes_from_candidate()` to check lock
- [ ] Modify `redistribute_votes()` to check lock and set new lock
- [ ] Add unit tests
- [ ] Add integration tests
- [ ] Configure lock period in runtime (201,600 blocks = 14 days)
- [ ] Update UI to show lock status and countdown
- [ ] Deploy to testnet for validation
- [ ] Deploy to mainnet

---

## 11. API Changes

### 11.1 New Query

```rust
/// Get remaining lock time for a voter's delegation to a candidate
/// Returns None if not locked, Some(blocks) if locked
fn get_vote_lock_remaining(voter: AccountId, candidate: AccountId) -> Option<BlockNumber>;
```

### 11.2 New Error

Extrinsics `try_remove_votes_from_candidate` and `redistribute_votes` may now return:
- `VotesStillLocked` - Votes are locked until block X

---

## 12. References

- `d9-node-voting/src/lib.rs` - Node voting pallet
- `d9-council-lock/src/lib.rs` - Council lock pallet (benefits from this fix)
- Issue #16 - Vote recycling vulnerability
