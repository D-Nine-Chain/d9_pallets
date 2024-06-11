## Using `ApiPromise`
- [Using `ApiPromise`](#using-apipromise)
  - [`proposal_fee`](#proposal_fee)
    - [Description](#description)
    - [How to Call](#how-to-call)
  - [`lock_proposals`](#lock_proposals)
    - [Description](#description-1)
    - [How to Call](#how-to-call-1)
  - [`lock_referendums`](#lock_referendums)
    - [Description](#description-2)
    - [How to Call](#how-to-call-2)
  - [`concluded_lock_referendums`](#concluded_lock_referendums)
    - [Description](#description-3)
    - [How to Call by Session and Account ID](#how-to-call-by-session-and-account-id)
    - [How to Call All Entries](#how-to-call-all-entries)
  - [`locked_accounts`](#locked_accounts)
    - [Description](#description-4)
    - [How to Call](#how-to-call-3)
  - [`propose_lock`](#propose_lock)
    - [Description](#description-5)
    - [How to Call](#how-to-call-4)
  - [`propose_unlock`](#propose_unlock)
    - [Description](#description-6)
    - [How to Call](#how-to-call-5)
  - [`vote_on_proposal`](#vote_on_proposal)
    - [Description](#description-7)
    - [How to Call](#how-to-call-6)

### `proposal_fee`
#### Description
This storage item holds the fee required to propose a lock or unlock action on an account.

#### How to Call
Retrieve the current proposal fee using the Polkadot.js API:

```javascript
const { ApiPromise, WsProvider } = require('@polkadot/api');

async function getProposalFee() {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const fee = await api.query.d9CouncilLock.proposalFee();
  console.log(`Proposal Fee: ${fee}`);
}

getProposalFee().catch(console.error);
```

### `lock_proposals`
#### Description
This storage map keeps track of lock decision proposals made for each account.

#### How to Call
Fetch the lock decision proposal for a specific account:

```javascript
async function getLockProposal(accountId) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const proposal = await api.query.d9CouncilLock.lockProposals(accountId);
  console.log(`Lock Proposal for ${accountId}: ${proposal}`);
}

getLockProposal('account_id_here').catch(console.error);
```

### `lock_referendums`
#### Description
This storage map holds active lock referendums for each account.

#### How to Call
Fetch the lock referendum for a specific account:

```javascript
async function getLockReferendum(accountId) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const referendum = await api.query.d9CouncilLock.lockReferendums(accountId);
  console.log(`Lock Referendum for ${accountId}: ${referendum}`);
}

getLockReferendum('account_id_here').catch(console.error);
```

### `concluded_lock_referendums`
#### Description
This storage NMap holds the concluded lock referendums, indexed by session and account ID.

#### How to Call by Session and Account ID
Fetch the concluded lock referendum for a specific session and account:

```javascript
async function getConcludedLockReferendum(sessionIndex, accountId) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const referendum = await api.query.d9CouncilLock.concludedLockReferendums(sessionIndex, accountId);
  console.log(`Concluded Lock Referendum for session ${sessionIndex} and account ${accountId}: ${referendum}`);
}

getConcludedLockReferendum('session_index_here', 'account_id_here').catch(console.error);
```

#### How to Call All Entries
Fetch all concluded lock referendums:

```javascript
async function getAllConcludedReferendums(api) {
  const entries = await api.query.d9CouncilLock.concludedLockReferendums.entries();

  const referendums = entries.map(([key, value]) => {
    const [sessionIndex, accountId] = key.args;
    return { sessionIndex, accountId, value };
  });

  return referendums;
}

async function main() {
  const api = await setup();
  const referendums = await getAllConcludedReferendums(api);

  referendums.forEach(({ sessionIndex, accountId, value }) => {
    console.log(`Session: ${sessionIndex}, AccountId: ${accountId}, Referendum: ${value}`);
  });
}

main().catch(console.error);
```

### `locked_accounts`
#### Description
This storage map holds the lock state of accounts.

#### How to Call
Fetch the lock state of a specific account:

```javascript
async function getLockedAccount(accountId) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const lockState = await api.query.d9CouncilLock.lockedAccounts(accountId);
  console.log(`Lock State for ${accountId}: ${lockState}`);
}

getLockedAccount('account_id_here').catch(console.error);
```

### `propose_lock`
#### Description
This extrinsic allows a user to propose a lock on an account by paying a fee.

#### How to Call
Propose a lock on an account:

```javascript
async function proposeLock(signer, accountToLock, proposalFee) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const unsub = await api.tx.d9CouncilLock.proposeLock(accountToLock, proposalFee)
    .signAndSend(signer, (result) => {
      if (result.status.isInBlock) {
        console.log(`Included at block hash ${result.status.asInBlock}`);
      } else if (result.status.isFinalized) {
        console.log(`Finalized block hash ${result.status.asFinalized}`);
        unsub();
      }
    });
}

const signer = ...; // Your keyring signer
proposeLock(signer, 'account_to_lock', 1000).catch(console.error);
```

### `propose_unlock`
#### Description
This extrinsic allows a user to propose an unlock on an account by paying a fee.

#### How to Call
Propose an unlock on an account:

```javascript
async function proposeUnlock(signer, accountToUnlock, proposalFee) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const unsub = await api.tx.d9CouncilLock.proposeUnlock(accountToUnlock, proposalFee)
    .signAndSend(signer, (result) => {
      if (result.status.isInBlock) {
        console.log(`Included at block hash ${result.status.asInBlock}`);
      } else if (result.status.isFinalized) {
        console.log(`Finalized block hash ${result.status.asFinalized}`);
        unsub();
      }
    });
}

const signer = ...; // Your keyring signer
proposeUnlock(signer, 'account_to_unlock', 1000).catch(console.error);
```

### `vote_on_proposal`
#### Description
This extrinsic allows a user to vote on an active lock proposal for a specific account.

#### How to Call
Vote on a lock proposal:

```javascript
async function voteOnProposal(signer, lockCandidate, assentOnDecision) {
  const provider = new WsProvider('wss://your-node-url');
  const api = await ApiPromise.create({ provider });

  const unsub = await api.tx.d9CouncilLock.voteOnProposal(lockCandidate, assentOnDecision)
    .signAndSend(signer, (result) => {
      if (result.status.isInBlock) {
        console.log(`Included at block hash ${result.status.asInBlock}`);
      } else if (result.status.isFinalized) {
        console.log(`Finalized block hash ${result.status.asFinalized}`);
        unsub();
      }
    });
}

const signer = ...; // Your keyring signer
voteOnProposal(signer, 'lock_candidate_account_id', true).catch(console.error);
```
