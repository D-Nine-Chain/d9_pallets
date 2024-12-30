#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
mod tests;
mod types;
use frame_support::BoundedVec;
pub use types::*;
pub type BoundedCallOf<T> = BoundedVec<u8, <T as Config>::MaxCallSize>;
#[frame_support::pallet]
pub mod pallet {
    use core::ops::Mul;

    use super::*;
    use frame_support::dispatch::{
        DispatchResult, DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo, PostDispatchInfo,
    };
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_system::RawOrigin;
    use sp_std::collections::btree_set::BTreeSet;

    const STORAGE_VERSION: frame_support::traits::StorageVersion =
        frame_support::traits::StorageVersion::new(1);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type MaxSignatories: Get<u32>;
        type MaxPendingCalls: Get<u32>;
        type MaxMultiSigsPerAccountId: Get<u32>;
        type RuntimeCall: Parameter
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
            + GetDispatchInfo
            + From<Call<Self>>;
        type MaxCallSize: Get<u32>;
    }

    /// the existent multi signature accounts
    ///
    /// key: multi signature account address value: multi signature account
    #[pallet::storage]
    #[pallet::getter(fn multi_signature_accounts)]
    pub type MultiSignatureAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, MultiSignatureAccount<T>, OptionQuery>;

    /// key: normal accounts value: vec of multi signature account ids that user is a member of
    #[pallet::storage]
    #[pallet::getter(fn user_multi_signature_accounts)]
    pub type UserMultiSigAccounts<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<T::AccountId, T::MaxMultiSigsPerAccountId>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// (creator, created)
        MultiSignatureAccountCreated(T::AccountId, T::AccountId),
        /// (updater, updated)
        MultiSignatureAccountUpdated(T::AccountId, T::AccountId),
        /// new transaction authors (author, call_id)
        NewCallAuthored(T::AccountId, [u8; 32]),
        /// add approval to transaction (multi signature account, approver)
        ApprovalAdded(T::AccountId, T::AccountId),
        /// remove approval from transaction (multi signature account, approver)
        ApprovalRemoved(T::AccountId, T::AccountId),
        /// (executor)
        CallExecuted([u8; 32]),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Signers list contains duplicates
        DuplicatesInList,
        /// Caller not in signatories
        CallerNotSignatory,
        /// Multi-sig account already exists
        MSAAlreadyExists,
        /// Multi-sig account not found
        MSANotFound,
        /// Must have at least 2 signers
        SignatoriesTooShort,
        /// Too many signers
        SignatoriesTooLong,
        /// Minimum approvals is out of range
        MinApprovalOutOfRange,
        /// Account is already an author
        AccountAlreadyAuthor,
        /// Authors list too long
        AuthorVecTooLong,
        /// Not an author
        AccountNotAuthor,
        /// Not a signatory
        AccountNotSignatory,
        /// Reached max possible authors
        AccountErrorMaxAuthors,
        /// Pending calls limit reached
        CallLimit,
        /// Approvals limit reached
        ApprovalsLimitReached,
        /// Failed to encode call
        CallEncodingFailure,
        /// This account hit its multi-sig limit
        AccountAtMultiSigLimit,
        /// Failed to create pending call
        FailedToCreatePendingCall,
        /// Call not found for multi-sig
        CallNotFound,
        /// Failed to decode call
        FailureDecodingCall,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// create a new multi signature account
        ///
        /// signatories must have cardinality of at least 2 and
        /// less than T::MaxSignatories, with all unique elements.
        /// origin must be an element of signatories and authors a
        /// subset of signatories.
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn create_multi_sig_account(
            origin: OriginFor<T>,
            signatories: Vec<T::AccountId>,
            authors: Option<Vec<T::AccountId>>,
            min_approving_signatories: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            // build and validate signatories bounded vec
            let mut bounded_signatories =
                BoundedVec::try_from(signatories).map_err(|_| Error::<T>::SignatoriesTooLong)?;
            bounded_signatories.sort();
            Self::validate_signatories(&bounded_signatories, &signer, min_approving_signatories)?;

            let authors = Self::prepare_authors(authors, &bounded_signatories)?;

            //
            let msa: MultiSignatureAccount<T> =
                MultiSignatureAccount::new(bounded_signatories, authors, min_approving_signatories)
                    .map_err(Error::<T>::from)?;
            let msa_address = msa.address.clone();
            Self::add_multi_sig_account_to_storage(msa)?;
            Self::deposit_event(Event::MultiSignatureAccountCreated(signer, msa_address));
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn author_a_call(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // validate
            let mut msa = MultiSignatureAccounts::<T>::get(&multi_sig_account)
                .ok_or(Error::<T>::MSANotFound)?;

            if !msa.is_author(&signer) {
                return Err(Error::<T>::AccountNotAuthor.into());
            }

            //prepare the call
            let pending_call = PendingCall::<T>::new(call, signer.clone())
                .map_err(|_| Error::<T>::FailedToCreatePendingCall)?;
            msa.add_call(pending_call.clone())
                .map_err(|_| Error::<T>::CallLimit)?;
            MultiSignatureAccounts::<T>::insert(&multi_sig_account, msa);
            Self::deposit_event(Event::NewCallAuthored(signer, pending_call.id));
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn add_approval(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            call_id: [u8; 32],
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let mut msa = MultiSignatureAccounts::<T>::get(&multi_sig_account)
                .ok_or(Error::<T>::MSANotFound)?;
            let is_signatory = msa.is_signatory(&signer);
            if !is_signatory {
                return Err(Error::<T>::AccountNotSignatory.into());
            }
            let mut pending_call = {
                let idx = msa
                    .pending_calls
                    .iter()
                    .position(|c| c.id == call_id)
                    .ok_or(Error::<T>::CallNotFound)?;
                let pending_call = msa.pending_calls.swap_remove(idx);
                pending_call
            };

            let approvals = pending_call
                .add_approval(signer.clone())
                .map_err(|_| Error::<T>::ApprovalsLimitReached)?;

            if approvals == msa.minimum_signatories {
                Self::execute_call(&pending_call, &mut msa).map(|_info| ())?;
                Self::deposit_event(Event::CallExecuted(pending_call.id.clone()));
            } else {
                msa.add_call(pending_call)
                    .map_err(|_| Error::<T>::CallLimit)?;
            }
            MultiSignatureAccounts::<T>::insert(&multi_sig_account, msa);
            Ok(().into())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn remove_approval(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            call_id: [u8; 32],
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let mut msa = MultiSignatureAccounts::<T>::get(&multi_sig_account)
                .ok_or(Error::<T>::MSANotFound)?;
            let is_signatory = msa.is_signatory(&signer);
            if !is_signatory {
                return Err(Error::<T>::AccountNotSignatory.into());
            }
            let mut pending_call = {
                let idx = msa
                    .pending_calls
                    .iter()
                    .position(|c| c.id == call_id)
                    .ok_or(Error::<T>::CallNotFound)?;
                let pending_call = msa.pending_calls.swap_remove(idx);
                pending_call
            };

            pending_call
                .remove_approval(signer.clone())
                .map_err(|_| Error::<T>::ApprovalsLimitReached)?;
            msa.add_call(pending_call)
                .map_err(|_| Error::<T>::CallLimit)?;
            MultiSignatureAccounts::<T>::insert(&multi_sig_account, msa);
            Ok(().into())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn adjust_min_approvals(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            new_min_approval: u32,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            MultiSignatureAccounts::<T>::try_mutate(&multi_sig_account, |msa_opt| {
                let msa_ref = msa_opt.as_mut().ok_or(Error::<T>::MSANotFound)?;

                if !msa_ref.is_author(&signer) {
                    return Err(Error::<T>::AccountNotAuthor.into());
                }

                if !(2..=(msa_ref.signatories.len() as u32)).contains(&new_min_approval) {
                    return Err(Error::<T>::MinApprovalOutOfRange.into());
                }

                msa_ref.minimum_signatories = new_min_approval;

                Ok(())
            })
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn remove_call(
            origin: OriginFor<T>,
            multi_sig_account: T::AccountId,
            call_id: [u8; 32],
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            MultiSignatureAccounts::<T>::try_mutate(&multi_sig_account, |msa_opt| {
                let msa_ref = msa_opt.as_mut().ok_or(Error::<T>::MSANotFound)?;
                if !msa_ref.is_author(&signer) {
                    return Err(Error::<T>::AccountNotAuthor.into());
                }
                let idx = msa_ref
                    .pending_calls
                    .iter()
                    .position(|c| c.id == call_id)
                    .ok_or(Error::<T>::CallNotFound)?;
                msa_ref.pending_calls.remove(idx);
                Ok::<(), Error<T>>(())
            })?;
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Validate the signatories list and minimum approvals
        fn validate_signatories(
            signatories: &BoundedVec<T::AccountId, T::MaxSignatories>,
            creator: &T::AccountId,
            min_approvals: u32,
        ) -> Result<(), Error<T>> {
            let signatories_len = signatories.len() as u32;
            if signatories_len < 2 {
                return Err(Error::<T>::SignatoriesTooShort);
            }
            if !(1..=signatories_len).contains(&min_approvals) {
                return Err(Error::<T>::MinApprovalOutOfRange);
            }
            let mut unique = BTreeSet::new();
            let mut found_creator = false;
            for signatory in signatories.iter() {
                if !unique.insert(signatory) {
                    return Err(Error::<T>::DuplicatesInList);
                }
                if signatory == creator {
                    found_creator = true;
                }
                if let Some(user_msas) = UserMultiSigAccounts::<T>::get(signatory) {
                    if user_msas.len() as u32 >= T::MaxMultiSigsPerAccountId::get() {
                        return Err(Error::<T>::AccountAtMultiSigLimit);
                    }
                }
            }
            if !found_creator {
                return Err(Error::<T>::CallerNotSignatory);
            }
            Ok(())
        }

        fn prepare_authors(
            authors_opt: Option<Vec<T::AccountId>>,
            sorted_signatories: &BoundedVec<T::AccountId, T::MaxSignatories>,
        ) -> Result<Option<BoundedVec<T::AccountId, T::MaxSignatories>>, Error<T>> {
            match authors_opt {
                Some(mut authors) => {
                    // If authors == signatories in size, treat it as "all are authors" → force None
                    if authors.len() == sorted_signatories.len() {
                        return Ok(None);
                    }
                    authors.sort();
                    Self::check_duplicates(&authors)?;
                    // Ensure each author is also a signatory
                    for author in &authors {
                        if sorted_signatories.as_slice().binary_search(author).is_err() {
                            return Err(Error::<T>::AccountNotAuthor);
                        }
                    }
                    // Attempt to convert into a bounded vec
                    let authors_bounded = BoundedVec::try_from(authors)
                        .map_err(|_| Error::<T>::AccountErrorMaxAuthors)?;
                    Ok(Some(authors_bounded))
                }
                None => Ok(None),
            }
        }
        ///  check for duplicates in sorted vec
        fn check_duplicates(account_ids: &Vec<T::AccountId>) -> Result<(), Error<T>> {
            let has_duplicates = !account_ids.windows(2).all(|pair| pair[0] != pair[1]);
            if has_duplicates {
                return Err(Error::<T>::DuplicatesInList);
            }
            Ok(())
        }

        fn add_multi_sig_account_to_storage(msa: MultiSignatureAccount<T>) -> Result<(), Error<T>> {
            if MultiSignatureAccounts::<T>::contains_key(&msa.address) {
                return Err(Error::<T>::MSAAlreadyExists.into());
            }
            //todo verify this mutate. check to see if you can explicitly define the type within the closure e.g. Option<BoundedVec<T::AccountId, T::MaxMultiSigsPerAccountId>>
            for signatory in msa.signatories.iter() {
                UserMultiSigAccounts::<T>::mutate(signatory, |user_msas_opt| {
                    if let Some(ref mut user_msas) = user_msas_opt {
                        user_msas.try_push(msa.address.clone()).unwrap();
                    } else {
                        let mut user_msas = BoundedVec::default();
                        user_msas.try_push(msa.address.clone()).unwrap();
                        *user_msas_opt = Some(user_msas);
                    }
                });
            }

            MultiSignatureAccounts::<T>::insert(msa.address.clone(), msa);

            Ok(())
        }

        fn execute_call(
            call: &PendingCall<T>,
            msa: &mut MultiSignatureAccount<T>,
        ) -> DispatchResultWithPostInfo {
            let decoded_call = call
                .decode_call()
                .map_err(|_| Error::<T>::FailureDecodingCall)?;
            let origin = RawOrigin::Signed(msa.address.clone()).into();
            decoded_call.dispatch(origin)
        }
    }
}
