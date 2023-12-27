use codec::Codec;
use frame_support::inherent::Vec;
sp_api::decl_runtime_apis! {
    pub trait D9BurnElectionApi<AccountId> where AccountId: Codec {
        fn get_sorted_candidates() -> Option<Vec<AccountId>>;
    }
}
