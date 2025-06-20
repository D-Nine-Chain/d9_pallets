#[cfg(test)]
mod tests {
    use crate::{PenaltyType, Severity};
    use codec::{Encode, Decode};

    #[test]
    fn penalty_type_encoding_works() {
        let penalty = PenaltyType::GovernanceLock;
        let encoded = penalty.encode();
        let decoded = PenaltyType::decode(&mut &encoded[..]).unwrap();
        assert_eq!(penalty, decoded);
    }

    #[test]
    fn severity_ordering_works() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }
}