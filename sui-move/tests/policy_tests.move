#[test_only]
module cput::policy_tests {
    use cput::policy;

    #[test]
    fun distribution_sums_to_total() {
        let total = 1_000_000_000u64;
        let d = policy::compute_distribution(total);
        let sum = policy::providers(&d)
            + policy::oracle(&d)
            + policy::treasury(&d)
            + policy::burn_reserve(&d);
        assert!(sum == total, 0);
    }

    #[test]
    fun fee_routing_sums_to_fee() {
        let fee = 500_000_000u64;
        let r = policy::compute_fee_routing(fee);
        let sum = policy::fee_burned(&r) + policy::fee_providers(&r) + policy::fee_treasury(&r);
        assert!(sum == fee, 0);
    }

    #[test]
    fun recommend_ceiling_low_util_decreases() {
        let current = 10_000_000_000u64;
        let next = policy::recommend_ceiling(current, 5000);
        assert!(next < current, 0);
    }

    #[test]
    fun recommend_ceiling_high_util_increases() {
        let current = 10_000_000_000u64;
        let next = policy::recommend_ceiling(current, 9500);
        assert!(next > current, 0);
    }
}
