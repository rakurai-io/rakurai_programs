use rakurai_integration::harness::TestEnv;

/// End-to-end flow across rakurai_activation, reward_distribution, and rakurai_tip_manager.
///
/// Prerequisite: `anchor build --no-idl` from the repo root (produces `target/deploy/*.so`).
///
/// Skipped (deferred): Merkle root upload, staker claim, RCA close.
#[tokio::test]
async fn end_to_end_rakurai_flow() {
    const TOTAL_REWARDS: u64 = 1_000_000;
    const TCA_RECORD: u64 = 500_000;
    const BCA_RECORD: u64 = 300_000;
    const TIP_LAMPORTS: u64 = 800_000;

    let mut env = TestEnv::start().await;

    env.phase1_init_configs().await;
    env.phase2_raa_enable().await;

    env.phase3_rca_and_revenue_shares().await;

    let rca_before = env.rca_lamports().await;
    env.phase4_simulate_turns(TOTAL_REWARDS, TCA_RECORD, BCA_RECORD, TIP_LAMPORTS)
        .await;

    let expected_rca_increase = TOTAL_REWARDS - (TOTAL_REWARDS * 500 / 10_000);
    assert!(
        env.rca_lamports().await >= rca_before + expected_rca_increase,
        "RCA should receive staker rewards minus block builder commission"
    );

    env.phase5_warp_epoch().await;
    env.phase6_fund_and_claim(TCA_RECORD, BCA_RECORD).await;
    env.phase7_close_revenue_shares().await;
}
