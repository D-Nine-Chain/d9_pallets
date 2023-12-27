use crate::pallet::Config;
sp_api::decl_runtime_apis! {
    pub trait D9BurnElectionApi<T: Config> {
        fn get_sorted_candidates() -> Option<Vec<T::AccountId>>;
    }
}
