pub enum Error<T> {
    /// a multi signature account with the same signatories already exists
    //note add error that related to the creation of multi signature accounts
    MultiSignatureAccountAlreadyExists,
    MultiSignatureAccountNotFound,
    PendingTransactionAlreadyExists,
    AccountError([u8; 32]),
}
