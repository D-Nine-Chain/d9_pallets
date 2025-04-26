## Using `ApiRx`

## extrinsics

[extrinsics documentation](extrinsics.md) (state changing functions)
- [Using `ApiRx`](#using-apirx)
- [extrinsics](#extrinsics)
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
without a `sessionIndex` argument the function return all the concluded referendums that have every happened. 
with a `sessionIndex` integer value it return just for that particular session if there are any. if there are none then it returns `null`

```javascript
function getConcludedReferendums(sessionIndex?: number) {
     return getApi$().pipe(
          switchMap(
               (api) => {
                    return api.query.councilLock.concludedLockReferendums.entries(sessionIndex)
               }
          ),
          map(
               (entries) => {
                    return entries.map(([storageKey, codec]) => {
                         return [storageKey.toHuman(), codec.toJSON()]
                    })
               }
          )
     )
}
```

### `lockDecisionProposals`
#### Description
get lock decision proposals (proposal to lock/unlock an account)

```javascript
function getLockProposals(accountId?: string) {
     return getApi$().pipe(
          switchMap(
               (api) => {
                    return api.query.councilLock.lockDecisionProposals.entries(accountId)
               }
          ),
          map(
               (entries) => {
                    return entries.map(([_, codec]) => {
                         return codec.toJSON()
                    })
               }
          )
     )
}
```
this second way will only work to get single values and the above way will work to get single or all. 

to check if a particular account has an active proposal do. returns `null` if no lockproposals for that account:
```javascript
function getLockProposals(accountId:string) {
     return getApi$().pipe(
          switchMap(
               (api) => {
                    return api.query.councilLock.lockDecisionProposals(accountId)
               }
          ),
          map(
               (entries) => {
                    return entries.map(([storageKey, codec]) => {
                         return codec.toJSON()
                    })
               }
          )
     )
}

```

### `lockReferendums`
#### Description
This storage map holds active lock referendums for each account.


```javascript
function getLockReferendums(accountId?:string) {
     return getApi$().pipe(
          switchMap(
               (api) => {
                    return api.query.councilLock.lockReferendums.entries(accountId)
               }
          ),
          map(
               (entries) => {
                    return entries.map(([storageKey, codec]) => {
                         return codec.toJSON()
                    })
               }
          )
     )
}

```

### `lockedAccounts`
#### Description
get locked account info. 

```javascript
function getLockedAccountData(accountId?: string) {
     return getApi$().pipe(
          switchMap(
               (api) => {
                    return api.query.councilLock.l.entries(accountId)
               }
          ),
          map(
               (entries) => {
                    return entries.map(([_, codec]) => {
                         return codec.toJSON()
                    })
               }
          )
     )
}
```

### `proposalFee`
#### Description
This storage item holds the fee required to propose a lock or unlock action on an account.

```javascript
function getProposalFee() {
     return getApi$().pipe(
          switchMap(
               (api) => {
                    return api.query.councilLock.proposalFee()
               }
          ),
          map(
               (feeCodec) => {
                    return feeCodec.toJSON()
               }
          )
     )
}
```


