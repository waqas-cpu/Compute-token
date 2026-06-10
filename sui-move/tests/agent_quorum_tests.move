#[test_only]
module cput::agent_quorum_tests {
    use cput::policy;

    #[test]
    fun spread_within_tolerance() {
        let spread = policy::spread_bps(100, 101);
        assert!(spread <= policy::consensus_tolerance_bps(), 0);
    }

    #[test]
    fun spread_exceeds_tolerance() {
        let spread = policy::spread_bps(100, 105);
        assert!(spread > policy::consensus_tolerance_bps(), 0);
    }

    #[test]
    fun consensus_average() {
        let c = policy::consensus_total(100, 100, 103);
        assert!(c == 101, 0);
    }

    #[test]
    fun agent_quorum_size_is_three() {
        assert!(policy::agent_quorum_size() == 3, 0);
    }
}
