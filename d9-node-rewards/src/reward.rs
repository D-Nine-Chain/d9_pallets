use frame_support::traits::tokens::Balance;

use substrate_fixed::{ FixedU128, types::extra::U30 };
pub type FixedBalance = FixedU128<U30>;

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Encode,
    Decode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen
)]
pub enum NodeTier {
    Super(SuperNodeSubTier),
    StandBy,
    Candidate,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Encode,
    Decode,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen
)]
pub enum SuperNodeSubTier {
    Upper,
    Middle,
    Lower,
}

#[derive(
    PartialEqNoBound,
    EqNoBound,
    CloneNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen
)]
#[scale_info(skip_type_params(T))]
struct TierRewardPools<T: Config> {
    supers: BalanceOf<T>,
    standbys: BalanceOf<T>,
    candidates: BalanceOf<T>,
}
impl TierRewardPools<T: Config> {
    fn new() -> Self {
        TierRewardPools {
            supers: 0,
            standbys: 0,
            candidates: 0,
        }
    }
    fn calculate_tier_reward_pools(&mut self, node_reward_pool: BalanceOf<T>) {
        let ten_percent = FixedBalance::from_num(10)
            .checked_div(FixedBalance::from_num(100))
            .unwrap();
        let session_rewards = ten_percent.saturating_mul(node_reward_pool).to_num::<BalanceOf<T>>();
        let supers_percent = FixedBalance::from_num(54)
            .checked_div(FixedBalance::from_num(100))
            .unwrap();
        let standbys_percent = FixedBalance::from_num(30)
            .checked_div(FixedBalance::from_num(100))
            .unwrap();
        let candidates_percent = FixedBalance::from_num(16)
            .checked_div(FixedBalance::from_num(100))
            .unwrap();

        self.supers = supers_percent.saturating_mul(session_rewards).to_num::<BalanceOf<T>>();
        self.standbys = standbys_percent.saturating_mul(session_rewards).to_num::<BalanceOf<T>>();
        self.candidates = candidates_percent
            .saturating_mul(session_rewards)
            .to_num::<BalanceOf<T>>();
    }
}
