### `proposeLock`

`accountId` here is the account that the user is proposing to lock
this function gets the fee (1000 D9) and includes it in the `proposeLock` function to automatically withdraw.

it would be better to check the balance first to see if there are sufficient funds to pay the proposal fee:

```typescript
  api.query.councilLock.proposalFee()
```

```typescript
function proposeLock(accountId: string): Observable<SubmittableExtrinsic<"rxjs", ISubmittableResult>> {
     return getApi$().pipe(
          switchMap(api =>
               api.query.councilLock.proposalFee().pipe(
                    map(
                         proposalFee => ({ api, proposalFee })
                    )
               )
          ),
          map(({ api, proposalFee }) => {
               return api.tx.councilLock.proposeLock(accountId, proposalFee);
          }),
     );
}
```


### `voteOnProposal`

`lockCandidate` would be retrieved from `lockReferendums` and you will send a boolean value `agreeToLock` of `true` if the user wishes to lock this account and `false` otherwise
```typescript
function voteOnProposal(lockCandidate: string, agreeToLock: boolean): Observable<SubmittableExtrinsic<"rxjs", ISubmittableResult>> {
     return getApi$().pipe(
          map(api => {
               return api.tx.councilLock.voteOnProposal(lockCandidate, agreeToLock);
          })
     )
}
```

the return type `ISubmittableResult` is just the same as the others extrinsics in the wallet app. 

