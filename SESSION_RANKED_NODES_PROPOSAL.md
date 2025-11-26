# Proposal: Session-Based Node Ranking Snapshot

**Issue:** #16
**Author:** D9 Core Team
**Status:** Draft
**Created:** 2024-11-27
**Affects:** `d9-node-voting`, `d9-council-lock`

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

The `check_is_council_member()` function in `d9-council-lock` queries rankings in real-time via `RankingProvider::get_ranked_nodes()`. Rankings are computed dynamically from `NodeAccumulativeVotes`, which updates immediately when votes are moved.

```rust
// d9-council-lock/src/lib.rs:458-466
fn check_is_council_member(account_id: &T::AccountId) -> Result<(), Error<T>> {
    let ranked_nodes = Self::get_ranked_nodes()?;  // Real-time computation
    if let Some(index) = ranked_nodes.iter().position(|x| x == account_id) {
        if index < T::VotingCouncilSize::get() as usize {
            return Ok(());
        }
    }
    return Err(Error::<T>::NotValidCouncilMember);
}
```

---

## 2. Proposed Solution

### 2.1 Overview

Implement session-based ranking snapshots in `d9-node-voting`. Rankings will only update at session boundaries, making them stable within a session and preventing mid-session manipulation.

### 2.2 Design Principles

1. **Single source of truth:** Node voting pallet owns the ranking snapshot
2. **Session-aligned stability:** Rankings are immutable within a session
3. **Minimal downstream impact:** Contracts and other pallets unaffected
4. **No storage migration required:** Additive change only

---

## 3. Technical Specification

### 3.1 New Storage Item

Add to `d9-node-voting/src/lib.rs`:

```rust
/// Snapshot of ranked nodes, updated at session boundaries.
/// Used by RankingProvider to return stable rankings within a session.
#[pallet::storage]
#[pallet::getter(fn session_ranked_nodes)]
pub type SessionRankedNodes<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;
```

### 3.2 Snapshot Update Logic

Update the snapshot at session start in `new_session()`:

```rust
fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
    // Update the session-stable ranking snapshot
    let sorted_candidates = Self::get_sorted_candidates();
    if let Some(ref candidates) = sorted_candidates {
        SessionRankedNodes::<T>::put(candidates.clone());
    }

    // ... existing logic ...
    sorted_candidates
}
```

### 3.3 RankingProvider Implementation Change

Modify `get_ranked_nodes()` to return the snapshot:

```rust
impl<T: Config> RankingProvider<T::AccountId> for Pallet<T> {
    fn get_ranked_nodes() -> Option<Vec<T::AccountId>> {
        let snapshot = SessionRankedNodes::<T>::get();
        if snapshot.is_empty() {
            // Fallback for first session after upgrade
            Self::get_sorted_candidates()
        } else {
            Some(snapshot)
        }
    }

    // ... other methods unchanged ...
}
```

### 3.4 Preserve Live Computation for Rewards

The existing `get_sorted_candidates()` and `get_sorted_candidates_with_votes()` functions remain unchanged and continue to compute live rankings. These are used for:

- Validator selection at session end
- Reward distribution (requires current vote counts)

---

## 4. Behavioral Changes

### 4.1 Before (Current Behavior)

```
User moves votes -> NodeAccumulativeVotes updates -> get_ranked_nodes() returns new ranking immediately
```

- Rankings are volatile within a session
- Vote manipulation can change council eligibility mid-referendum

### 4.2 After (Proposed Behavior)

```
User moves votes -> NodeAccumulativeVotes updates -> get_ranked_nodes() returns session snapshot (unchanged)
Session boundary -> Snapshot updates -> get_ranked_nodes() returns new ranking
```

- Rankings are stable within a session
- Council eligibility is fixed for the duration of a referendum
- Vote changes take effect at next session boundary

---

## 5. Downstream Consequences

### 5.1 Affected: `d9-council-lock`

| Function | Impact |
|----------|--------|
| `check_is_council_member()` | Now uses stable snapshot; council membership fixed per session |
| `check_nominator()` | Now uses stable snapshot; nominator eligibility fixed per session |
| `vote_in_referendum()` | Voters determined by session-start ranking, not real-time |

**Security improvement:** Eliminates vote recycling attack.

### 5.2 Affected: User Experience

| Scenario | Before | After |
|----------|--------|-------|
| User moves votes to push node into top 27 | Immediate council eligibility | Eligibility at next session |
| User withdraws votes from top 27 node | Immediate loss of council status | Status retained until session end |
| Node drops out of top 27 mid-session | Cannot vote in new referendums | Can still vote until session end |

**User expectation change:** Vote redistribution effects are delayed by up to one session.

### 5.3 NOT Affected: Contracts

| Contract | Reason |
|----------|--------|
| `node-reward` | Receives pre-sorted data via `update_rewards()` from pallet |
| `d9-burn-mining` | Only uses `get_ancestors()`, no ranking queries |
| `main-pool` | Only uses `get_ancestors()`, no ranking queries |
| `merchant-mining` | Only uses `get_ancestors()`, no ranking queries |

### 5.4 NOT Affected: Reward Distribution

Reward distribution at session end continues to use live `get_sorted_candidates_with_votes()`, ensuring rewards reflect actual vote counts at distribution time.

### 5.5 NOT Affected: Validator Selection

Validator selection already occurs at session boundaries and uses the live sorted list, which is then stored as the new snapshot.

---

## 6. Migration & Deployment

### 6.1 Storage Migration

**Not required.** This is an additive change:

- New `SessionRankedNodes` storage initializes to empty `Vec`
- Existing storage items unchanged
- Fallback logic handles empty snapshot gracefully

### 6.2 Deployment Sequence

1. Deploy runtime upgrade with new pallet code
2. On first `new_session()` after upgrade, snapshot is populated
3. Until first session boundary, fallback returns live computation
4. After first session boundary, snapshot is active

### 6.3 Storage Version

Optional bump to `STORAGE_VERSION` for documentation purposes:

```rust
const STORAGE_VERSION: frame_support::traits::StorageVersion =
    frame_support::traits::StorageVersion::new(2);  // Was 1
```

---

## 7. Testing Requirements

### 7.1 Unit Tests

1. **Snapshot population:** Verify `SessionRankedNodes` updates at session boundary
2. **Snapshot stability:** Verify `get_ranked_nodes()` returns same result throughout session
3. **Vote changes ignored:** Verify mid-session vote redistribution doesn't affect `get_ranked_nodes()`
4. **Fallback behavior:** Verify empty snapshot falls back to live computation

### 7.2 Integration Tests

1. **Council voting:** Verify only session-start top 27 can vote on referendums
2. **Attack prevention:** Verify vote recycling attack no longer works
3. **Session transition:** Verify new rankings take effect after session change

### 7.3 Scenario Tests

```
Scenario: Vote recycling attack prevention
  Given Node A is rank 5 at session start
  And Node B is rank 30 at session start
  And a referendum is active
  When Node A votes on the referendum
  And supporter moves votes from Node A to Node B
  Then Node B should NOT be able to vote (still rank 30 in snapshot)
  And Node A should still be able to vote (still rank 5 in snapshot)
```

---

## 8. Risks & Mitigations

### 8.1 Risk: Stale Rankings

**Concern:** A node that loses significant support mid-session retains council privileges.

**Mitigation:** Session duration is short enough that this is acceptable. The alternative (real-time rankings) enables vote manipulation attacks which is worse.

### 8.2 Risk: First Session After Upgrade

**Concern:** Empty snapshot before first session boundary.

**Mitigation:** Fallback to live computation ensures continuity. After one session, snapshot is active.

### 8.3 Risk: Inconsistent State

**Concern:** Live vote counts and snapshot rankings diverge.

**Mitigation:** This is intentional and by design. Live counts are used for rewards; snapshot is used for governance eligibility. Both serve their purpose correctly.

---

## 9. Implementation Checklist

- [ ] Add `SessionRankedNodes` storage item to `d9-node-voting`
- [ ] Update `new_session()` to populate snapshot
- [ ] Modify `RankingProvider::get_ranked_nodes()` to return snapshot with fallback
- [ ] Add unit tests for snapshot behavior
- [ ] Add integration tests for attack prevention
- [ ] Update storage version (optional)
- [ ] Document behavioral changes for node operators
- [ ] Deploy to testnet for validation
- [ ] Deploy to mainnet

---

## 10. References

- `d9-node-voting/src/lib.rs` - Node voting pallet
- `d9-council-lock/src/lib.rs` - Council lock pallet
- `d9-chain-extension/lib.rs` - Chain extension for contracts
- Commit `f28d31b` - Previous fix for validator reward distribution sorting
