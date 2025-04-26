## Using `ApiPromise`
- [Using `ApiPromise`](#using-apipromise)
  - [`concludedLockReferendums`](#concludedlockreferendums)
    - [Description](#description)
  - [`lockDecisionProposals`](#lockdecisionproposals)
    - [Description](#description-1)
  - [`lockReferendums`](#lockreferendums)
    - [Description](#description-2)
  - [`lockedAccounts`](#lockedaccounts)
    - [Description](#description-3)
  - [`proposalFee`](#proposalfee)
    - [Description](#description-4)

### `concludedLockReferendums`
#### Description
Without a `sessionIndex` argument, the function returns all the concluded referendums that have ever happened. With a `sessionIndex` integer value, it returns just for that particular session if there are any. If there are none, it returns `null`.

```javascript
async function getConcludedReferendums(sessionIndex) {
     const api = await getApi();
     const entries = await api.query.councilLock.concludedLockReferendums.entries(sessionIndex);
     return entries.map(([storageKey, codec]) => {
          return [storageKey.toHuman(), codec.toJSON()];
     });
}
```

### `lockDecisionProposals`
#### Description
Get lock decision proposals (proposal to lock/unlock an account).

```javascript
async function getLockProposals(accountId) {
     const api = await getApi();
     const entries = await api.query.councilLock.lockDecisionProposals.entries(accountId);
     return entries.map(([_, codec]) => {
          return codec.toJSON();
     });
}
```
To check if a particular account has an active proposal, use the following. Returns `null` if no lock proposals for that account:

```javascript
async function getLockProposals(accountId) {
     const api = await getApi();
     const entries = await api.query.councilLock.lockDecisionProposals(accountId);
     return entries.map(([storageKey, codec]) => {
          return codec.toJSON();
     });
}
```

### `lockReferendums`
#### Description
This storage map holds active lock referendums for each account.

```javascript
async function getLockReferendums(accountId) {
     const api = await getApi();
     const entries = await api.query.councilLock.lockReferendums.entries(accountId);
     return entries.map(([storageKey, codec]) => {
          return codec.toJSON();
     });
}
```

### `lockedAccounts`
#### Description
Get locked account info.

```javascript
async function getLockedAccountData(accountId) {
     const api = await getApi();
     const entries = await api.query.councilLock.lockedAccounts.entries(accountId);
     return entries.map(([_, codec]) => {
          return codec.toJSON();
     });
}
```

### `proposalFee`
#### Description
This storage item holds the fee required to propose a lock or unlock action on an account.

```javascript
async function getProposalFee() {
     const api = await getApi();
     const feeCodec = await api.query.councilLock.proposalFee();
     return feeCodec.toJSON();
}
```