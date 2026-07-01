use {
    crate::vote::mock_vote_account,
    anchor_lang::AccountDeserialize,
    rakurai_activation::{
        sdk::{
            derive_activation_account_address, derive_config_account_address,
            instruction::{
                initialize_ix as init_activation_config_ix,
                initialize_rakurai_activation_account_ix,
                update_rakurai_activation_approval_ix,
                InitializeArgs as InitActivationArgs,
                InitializeAccounts as InitActivationAccounts,
                InitializeRakuraiActivationAccountArgs,
                InitializeRakuraiActivationAccountAccounts,
                UpdateRakuraiActivationApprovalArgs,
                UpdateRakuraiActivationApprovalAccounts,
            },
        },
        state::RakuraiActivationAccount,
    },
    rakurai_tip_manager::{
        sdk::{
            derive_rakurai_tip_collection_address, derive_rakurai_tip_manager_config_account_address,
            derive_rakurai_tip_payment_account_pdas,
            instruction::{
                change_block_builder_ix, change_tip_receiver_ix, initialize_rakurai_tip_manager_ix,
                ChangeBlockBuilderAccounts, ChangeBlockBuilderArgs, ChangeTipReceiverAccounts,
                InitializeRakuraiTipManagerAccounts, InitializeRakuraiTipManagerArgs,
            },
        },
        RakuraiTipManagerBumps, RAKURAI_REVENUE_NAME,
    },
    reward_distribution::{
        sdk::{
            derive_backrun_collection_account_address, derive_config_account_address as derive_rd_config,
            derive_reward_collection_account_address,
            instruction::{
                claim_revenue_ix, close_revenue_share_account_ix, initialize_ix as init_rd_config_ix,
                initialize_reward_collection_account_ix, initialize_revenue_share_account_ix,
                record_revenue_ix, transfer_staker_rewards_ix, update_config_ix,
                update_revenue_share_config_ix, ClaimRevenueArgs, ClaimRevenueShareAccounts,
                CloseRevenueShareAccountAccounts, InitializeArgs as InitRdArgs,
                InitializeAccounts as InitRdAccounts, InitializeRewardCollectionAccountArgs,
                InitializeRewardCollectionAccountAccounts, InitializeRevenueShareAccountArgs,
                InitializeRevenueShareAccountAccounts, RecordRevenueArgs, RecordRevenueShareAccounts,
                TransferStakerRewardsArgs, TransferStakerRewardsAccounts, UpdateConfigArgs,
                UpdateConfigAccounts, UpdateRevenueShareConfigArgs, UpdateRevenueShareConfigAccounts,
            },
        },
        state::{RevenueKind, RevenueShareAccount, RewardDistributionConfigAccount},
    },
    solana_program::system_program,
    solana_program_test::{ProgramTest, ProgramTestContext},
    solana_sdk::{
        clock::Clock,
        instruction::Instruction,
        pubkey::Pubkey,
        signature::{EncodableKey, Keypair, Signer},
        system_instruction,
        sysvar,
        transaction::Transaction,
    },
    std::path::PathBuf,
};

fn program_id_from_keypair_file(relative_path: &str) -> Pubkey {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../../target/deploy").join(relative_path);
    Keypair::read_from_file(&path)
        .unwrap_or_else(|e| panic!("read keypair {}: {e}", path.display()))
        .pubkey()
}

const AIRDROP_LAMPORTS: u64 = 50_000_000_000;

fn clone_kp(kp: &Keypair) -> Keypair {
    Keypair::from_bytes(&kp.to_bytes()).unwrap()
}

pub struct TestEnv {
    pub context: ProgramTestContext,
    pub activation_id: Pubkey,
    pub rd_id: Pubkey,
    pub tm_id: Pubkey,
    pub payer: Keypair,
    pub validator: Keypair,
    pub rakurai_bb: Keypair,
    pub revenue_manager: Keypair,
    pub record_authority: Keypair,
    pub external_settler: Keypair,
    pub vote_account: Pubkey,
    pub activation_config: Pubkey,
    pub rd_config: Pubkey,
    pub tm_config: Pubkey,
    pub raa: Pubkey,
    pub tca: Pubkey,
    pub bca: Pubkey,
    pub rca: Pubkey,
    pub tip_pdas: Vec<Pubkey>,
    pub creation_epoch: u64,
    pub tca_commission_bps: u16,
}

impl TestEnv {
    pub async fn start() -> Self {
        let activation_id = program_id_from_keypair_file("rakurai_activation-keypair.json");
        let rd_id = program_id_from_keypair_file("reward_distribution-keypair.json");
        let tm_id = program_id_from_keypair_file("rakurai_tip_manager-keypair.json");

        let validator = Keypair::new();
        let vote_account = Pubkey::new_unique();

        let mut program_test = ProgramTest::default();
        program_test.prefer_bpf(true);
        program_test.add_program("rakurai_activation", activation_id, None);
        program_test.add_program("reward_distribution", rd_id, None);
        program_test.add_program("rakurai_tip_manager", tm_id, None);
        program_test.set_compute_max_units(1_400_000);
        program_test.add_genesis_account(
            vote_account,
            mock_vote_account(&validator.pubkey(), AIRDROP_LAMPORTS / 10),
        );

        let mut context = program_test.start_with_context().await;
        let payer = Keypair::from_bytes(&context.payer.to_bytes()).unwrap();

        let mut env = Self {
            context,
            activation_id,
            rd_id,
            tm_id,
            payer,
            validator,
            rakurai_bb: Keypair::new(),
            revenue_manager: Keypair::new(),
            record_authority: Keypair::new(),
            external_settler: Keypair::new(),
            vote_account,
            activation_config: Pubkey::default(),
            rd_config: Pubkey::default(),
            tm_config: Pubkey::default(),
            raa: Pubkey::default(),
            tca: Pubkey::default(),
            bca: Pubkey::default(),
            rca: Pubkey::default(),
            tip_pdas: Vec::new(),
            creation_epoch: 0,
            tca_commission_bps: 1234,
        };

        env.fund_keypairs().await;
        env
    }

    async fn fund_keypairs(&mut self) {
        let payer = clone_kp(&self.payer);
        let recipients = [
            self.validator.pubkey(),
            self.rakurai_bb.pubkey(),
            self.revenue_manager.pubkey(),
            self.record_authority.pubkey(),
            self.external_settler.pubkey(),
        ];
        for pk in recipients {
            self.transfer_lamports(&payer, &pk, AIRDROP_LAMPORTS).await;
        }
    }

    pub async fn current_epoch(&mut self) -> u64 {
        let account = self
            .context
            .banks_client
            .get_account(sysvar::clock::id())
            .await
            .unwrap()
            .unwrap();
        let clock: Clock = bincode::deserialize(&account.data).unwrap();
        clock.epoch
    }

    async fn transfer_lamports(&mut self, from: &Keypair, to: &Pubkey, lamports: u64) {
        self.process_ixs(
            &[system_instruction::transfer(&from.pubkey(), to, lamports)],
            &[from],
        )
        .await;
    }

    pub async fn process_ixs(&mut self, ixs: &[Instruction], signers: &[&Keypair]) {
        let blockhash = self
            .context
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap();

        let payer_pk = self.payer.pubkey();
        let mut unique_signers: Vec<&Keypair> = Vec::new();
        if !signers.iter().any(|k| k.pubkey() == payer_pk) {
            unique_signers.push(&self.payer);
        }
        for signer in signers {
            if !unique_signers.iter().any(|k| k.pubkey() == signer.pubkey()) {
                unique_signers.push(signer);
            }
        }

        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&payer_pk),
            &unique_signers,
            blockhash,
        );
        self.context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_or_else(|e| panic!("transaction failed: {e:?}"));
    }

    pub async fn phase1_init_configs(&mut self) {
        let activation_id = self.activation_id;
        let rd_id = self.rd_id;
        let tm_id = self.tm_id;
        let payer = clone_kp(&self.payer);
        let rakurai_bb_pk = self.rakurai_bb.pubkey();

        let (activation_config, activation_config_bump) =
            derive_config_account_address(&activation_id);
        let (rd_config, rd_config_bump) = derive_rd_config(&rd_id);
        let (tm_config, _) = derive_rakurai_tip_manager_config_account_address(&tm_id);
        let tip_pdas: Vec<Pubkey> = derive_rakurai_tip_payment_account_pdas(&tm_id)
            .into_iter()
            .map(|(pk, _)| pk)
            .collect();

        self.activation_config = activation_config;
        self.rd_config = rd_config;
        self.tm_config = tm_config;
        self.tip_pdas = tip_pdas;

        self.process_ixs(
            &[init_activation_config_ix(
                activation_id,
                InitActivationArgs {
                    authority: payer.pubkey(),
                    block_builder_authority: rakurai_bb_pk,
                    block_builder_commission_account: rakurai_bb_pk,
                    block_builder_commission_bps: 500,
                    bump: activation_config_bump,
                },
                InitActivationAccounts {
                    config: activation_config,
                    system_program: system_program::id(),
                    initializer: payer.pubkey(),
                },
            )],
            &[&payer],
        )
        .await;

        self.process_ixs(
            &[init_rd_config_ix(
                rd_id,
                InitRdArgs {
                    authority: payer.pubkey(),
                    num_epochs_valid: 4,
                    max_commission_bps: 10_000,
                    block_builder_commission_on_mev_commission_enabled: false,
                    bump: rd_config_bump,
                },
                InitRdAccounts {
                    config: rd_config,
                    system_program: system_program::id(),
                    initializer: payer.pubkey(),
                },
            )],
            &[&payer],
        )
        .await;

        let mut rd_cfg = self.fetch_rd_config().await;
        rd_cfg.revenue_manager_authority = Some(self.revenue_manager.pubkey());
        self.process_ixs(
            &[update_config_ix(
                rd_id,
                UpdateConfigArgs { new_config: rd_cfg },
                UpdateConfigAccounts {
                    config: rd_config,
                    authority: payer.pubkey(),
                },
            )],
            &[&payer],
        )
        .await;

        let tip_bumps = self.derive_tip_bumps();
        self.process_ixs(
            &[initialize_rakurai_tip_manager_ix(
                tm_id,
                InitializeRakuraiTipManagerArgs { bumps: tip_bumps },
                InitializeRakuraiTipManagerAccounts {
                    tip_manager_config: tm_config,
                    rakurai_tip_account_0: self.tip_pdas[0],
                    rakurai_tip_account_1: self.tip_pdas[1],
                    rakurai_tip_account_2: self.tip_pdas[2],
                    rakurai_tip_account_3: self.tip_pdas[3],
                    rakurai_tip_account_4: self.tip_pdas[4],
                    rakurai_tip_account_5: self.tip_pdas[5],
                    rakurai_tip_account_6: self.tip_pdas[6],
                    rakurai_tip_account_7: self.tip_pdas[7],
                    system_program: system_program::id(),
                    payer: payer.pubkey(),
                },
            )],
            &[&payer],
        )
        .await;

        self.process_ixs(
            &[change_block_builder_ix(
                tm_id,
                ChangeBlockBuilderArgs {
                    block_builder_commission_bps: 500,
                },
                ChangeBlockBuilderAccounts {
                    tip_manager_config: tm_config,
                    validator_tip_receiver_account: payer.pubkey(),
                    old_block_builder: payer.pubkey(),
                    new_block_builder: rakurai_bb_pk,
                    rakurai_tip_account_0: self.tip_pdas[0],
                    rakurai_tip_account_1: self.tip_pdas[1],
                    rakurai_tip_account_2: self.tip_pdas[2],
                    rakurai_tip_account_3: self.tip_pdas[3],
                    rakurai_tip_account_4: self.tip_pdas[4],
                    rakurai_tip_account_5: self.tip_pdas[5],
                    rakurai_tip_account_6: self.tip_pdas[6],
                    rakurai_tip_account_7: self.tip_pdas[7],
                    signer: payer.pubkey(),
                },
            )],
            &[&payer],
        )
        .await;
    }

    fn derive_tip_bumps(&self) -> RakuraiTipManagerBumps {
        let (_, tip_manager_config) =
            derive_rakurai_tip_manager_config_account_address(&self.tm_id);
        let tips = derive_rakurai_tip_payment_account_pdas(&self.tm_id);
        RakuraiTipManagerBumps {
            tip_manager_config,
            rakurai_tip_account_0: tips[0].1,
            rakurai_tip_account_1: tips[1].1,
            rakurai_tip_account_2: tips[2].1,
            rakurai_tip_account_3: tips[3].1,
            rakurai_tip_account_4: tips[4].1,
            rakurai_tip_account_5: tips[5].1,
            rakurai_tip_account_6: tips[6].1,
            rakurai_tip_account_7: tips[7].1,
        }
    }

    pub async fn phase2_raa_enable(&mut self) {
        let activation_id = self.activation_id;
        let activation_config = self.activation_config;
        let vote_account = self.vote_account;
        let validator = clone_kp(&self.validator);
        let rakurai_bb = clone_kp(&self.rakurai_bb);

        let (raa, raa_bump) =
            derive_activation_account_address(&activation_id, &validator.pubkey());
        self.raa = raa;

        self.process_ixs(
            &[initialize_rakurai_activation_account_ix(
                activation_id,
                InitializeRakuraiActivationAccountArgs {
                    block_reward_commission_bps: 1_000,
                    bump: raa_bump,
                },
                InitializeRakuraiActivationAccountAccounts {
                    config: activation_config,
                    activation_account: raa,
                    system_program: system_program::id(),
                    validator_vote_account: vote_account,
                    validator_identity_account: validator.pubkey(),
                    signer: validator.pubkey(),
                },
            )],
            &[&validator],
        )
        .await;

        self.process_ixs(
            &[update_rakurai_activation_approval_ix(
                activation_id,
                UpdateRakuraiActivationApprovalArgs {
                    grant_approval: true,
                    hash: Some([7u8; 64]),
                },
                UpdateRakuraiActivationApprovalAccounts {
                    config: activation_config,
                    activation_account: raa,
                    validator_identity_account: validator.pubkey(),
                    signer: rakurai_bb.pubkey(),
                },
            )],
            &[&rakurai_bb],
        )
        .await;

        assert!(
            self.fetch_raa().await.is_enabled,
            "RAA should be enabled after Rakurai approval"
        );
    }

    pub async fn phase3_rca_and_revenue_shares(&mut self) {
        self.creation_epoch = self.current_epoch().await;

        let rd_id = self.rd_id;
        let rd_config = self.rd_config;
        let vote_account = self.vote_account;
        let raa = self.raa;
        let validator = clone_kp(&self.validator);
        let revenue_manager = clone_kp(&self.revenue_manager);

        let (rca, rca_bump) = derive_reward_collection_account_address(
            &rd_id,
            &vote_account,
            self.creation_epoch,
        );
        self.rca = rca;

        self.process_ixs(
            &[initialize_reward_collection_account_ix(
                rd_id,
                InitializeRewardCollectionAccountArgs {
                    merkle_root_upload_authority: validator.pubkey(),
                    block_reward_commission_bps: 1_000,
                    block_builder_commission_account: self.rakurai_bb.pubkey(),
                    block_builder_commission_bps: 500,
                    bump: rca_bump,
                },
                InitializeRewardCollectionAccountAccounts {
                    config: rd_config,
                    reward_collection_account: rca,
                    rakurai_activation_account: raa,
                    system_program: system_program::id(),
                    validator_vote_account: vote_account,
                    signer: validator.pubkey(),
                },
            )],
            &[&validator],
        )
        .await;

        let (tca, tca_bump) = derive_rakurai_tip_collection_address(&rd_id, &vote_account);
        self.tca = tca;

        self.process_ixs(
            &[initialize_revenue_share_account_ix(
                rd_id,
                InitializeRevenueShareAccountArgs {
                    share_kind: RevenueKind::Tip,
                    name: RAKURAI_REVENUE_NAME,
                    record_authority: self.record_authority.pubkey(),
                    max_epoch_entries: 4,
                    commission_bps: 0,
                    commission_account: self.rakurai_bb.pubkey(),
                    bump: tca_bump,
                },
                InitializeRevenueShareAccountAccounts {
                    revenue_share_account: tca,
                    config: rd_config,
                    validator_vote_account: vote_account,
                    payer: revenue_manager.pubkey(),
                    system_program: system_program::id(),
                },
            )],
            &[&revenue_manager],
        )
        .await;

        let bca_name = padded_name("BackrunPartner");
        let (bca, bca_bump) =
            derive_backrun_collection_account_address(&rd_id, &bca_name, &vote_account);
        self.bca = bca;

        self.process_ixs(
            &[initialize_revenue_share_account_ix(
                rd_id,
                InitializeRevenueShareAccountArgs {
                    share_kind: RevenueKind::Backrun,
                    name: bca_name,
                    record_authority: self.record_authority.pubkey(),
                    max_epoch_entries: 4,
                    commission_bps: 500,
                    commission_account: self.rakurai_bb.pubkey(),
                    bump: bca_bump,
                },
                InitializeRevenueShareAccountAccounts {
                    revenue_share_account: bca,
                    config: rd_config,
                    validator_vote_account: vote_account,
                    payer: revenue_manager.pubkey(),
                    system_program: system_program::id(),
                },
            )],
            &[&revenue_manager],
        )
        .await;

        self.process_ixs(
            &[update_revenue_share_config_ix(
                rd_id,
                UpdateRevenueShareConfigArgs {
                    commission_bps: self.tca_commission_bps,
                    commission_account: self.rakurai_bb.pubkey(),
                    convert_to_block_rewards: false,
                },
                UpdateRevenueShareConfigAccounts {
                    revenue_share_account: tca,
                    config: rd_config,
                    manager_authority: revenue_manager.pubkey(),
                },
            )],
            &[&revenue_manager],
        )
        .await;
    }

    pub async fn phase4_simulate_turns(
        &mut self,
        total_rewards: u64,
        tca_record: u64,
        bca_record: u64,
        tip_lamports: u64,
    ) {
        let rd_id = self.rd_id;
        let tm_id = self.tm_id;
        let rca = self.rca;
        let tca = self.tca;
        let bca = self.bca;
        let raa = self.raa;
        let tm_config = self.tm_config;
        let vote_account = self.vote_account;
        let creation_epoch = self.creation_epoch;
        let validator = clone_kp(&self.validator);
        let record_authority = clone_kp(&self.record_authority);
        let external_settler = clone_kp(&self.external_settler);
        let rakurai_bb = self.rakurai_bb.pubkey();
        let payer = self.payer.pubkey();
        let tip_pdas = self.tip_pdas.clone();

        self.process_ixs(
            &[transfer_staker_rewards_ix(
                rd_id,
                TransferStakerRewardsArgs { total_rewards },
                TransferStakerRewardsAccounts {
                    block_builder_commission_account: rakurai_bb,
                    reward_collection_account: rca,
                    system_program: system_program::id(),
                    signer: validator.pubkey(),
                },
            )],
            &[&validator],
        )
        .await;

        self.process_ixs(
            &[record_revenue_ix(
                rd_id,
                RecordRevenueArgs { amount: tca_record },
                RecordRevenueShareAccounts {
                    revenue_share_account: tca,
                    record_authority: record_authority.pubkey(),
                },
            )],
            &[&record_authority],
        )
        .await;

        self.process_ixs(
            &[record_revenue_ix(
                rd_id,
                RecordRevenueArgs { amount: bca_record },
                RecordRevenueShareAccounts {
                    revenue_share_account: bca,
                    record_authority: record_authority.pubkey(),
                },
            )],
            &[&record_authority],
        )
        .await;

        if tip_lamports > 0 {
            self.transfer_lamports(&external_settler, &tip_pdas[0], tip_lamports)
                .await;
            let bb_before = self.lamports(&rakurai_bb).await;
            self.process_ixs(
                &[change_tip_receiver_ix(
                    tm_id,
                    rakurai_tip_manager::sdk::instruction::ChangeTipReceiverArgs,
                    ChangeTipReceiverAccounts {
                        tip_manager_config: tm_config,
                        rakurai_activation_account: raa,
                        validator_vote_account: vote_account,
                        old_tip_receiver: payer,
                        reward_distribution_program: rd_id,
                        new_tip_receiver: tca,
                        block_builder_commission_account: rakurai_bb,
                        rakurai_tip_account_0: tip_pdas[0],
                        rakurai_tip_account_1: tip_pdas[1],
                        rakurai_tip_account_2: tip_pdas[2],
                        rakurai_tip_account_3: tip_pdas[3],
                        rakurai_tip_account_4: tip_pdas[4],
                        rakurai_tip_account_5: tip_pdas[5],
                        rakurai_tip_account_6: tip_pdas[6],
                        rakurai_tip_account_7: tip_pdas[7],
                        signer: validator.pubkey(),
                    },
                )],
                &[&validator],
            )
            .await;
            assert!(
                self.lamports(&rakurai_bb).await > bb_before,
                "tip drain should credit block builder commission account"
            );
        }

        let tca_state = self.fetch_revenue_share(tca).await;
        let entry = tca_state
            .ledger
            .entries
            .iter()
            .find(|e| e.epoch == creation_epoch)
            .expect("TCA ledger entry for creation epoch");
        assert_eq!(entry.amount, tca_record);
        assert!(!entry.claimed);

        let bca_state = self.fetch_revenue_share(bca).await;
        let bca_entry = bca_state
            .ledger
            .entries
            .iter()
            .find(|e| e.epoch == creation_epoch)
            .expect("BCA ledger entry for creation epoch");
        assert_eq!(bca_entry.amount, bca_record);
        assert!(!bca_entry.claimed);
    }

    pub async fn phase5_warp_epoch(&mut self) {
        let schedule = self.context.genesis_config().epoch_schedule.clone();
        let target_slot = schedule.first_normal_slot + schedule.slots_per_epoch + 1;
        self.context.warp_to_slot(target_slot).unwrap();
        let epoch = self.current_epoch().await;
        assert!(
            epoch > self.creation_epoch,
            "epoch should advance past creation epoch {} (got {epoch})",
            self.creation_epoch
        );
    }

    pub async fn phase6_fund_and_claim(&mut self, tca_amount: u64, bca_amount: u64) {
        let rd_id = self.rd_id;
        let tca = self.tca;
        let bca = self.bca;
        let creation_epoch = self.creation_epoch;
        let revenue_manager = clone_kp(&self.revenue_manager);
        let external_settler = clone_kp(&self.external_settler);
        let rakurai_bb = self.rakurai_bb.pubkey();
        let validator = self.validator.pubkey();
        let tca_commission_bps = self.tca_commission_bps;

        let commission_before = self.lamports(&rakurai_bb).await;
        let validator_before = self.lamports(&validator).await;

        self.transfer_lamports(&external_settler, &tca, tca_amount)
            .await;
        self.process_ixs(
            &[claim_revenue_ix(
                rd_id,
                ClaimRevenueArgs { epoch: creation_epoch },
                ClaimRevenueShareAccounts {
                    revenue_share_account: tca,
                    commission_account: rakurai_bb,
                    validator_identity: validator,
                    manager_authority: revenue_manager.pubkey(),
                },
            )],
            &[&revenue_manager],
        )
        .await;

        self.transfer_lamports(&external_settler, &bca, bca_amount)
            .await;
        self.process_ixs(
            &[claim_revenue_ix(
                rd_id,
                ClaimRevenueArgs { epoch: creation_epoch },
                ClaimRevenueShareAccounts {
                    revenue_share_account: bca,
                    commission_account: rakurai_bb,
                    validator_identity: validator,
                    manager_authority: revenue_manager.pubkey(),
                },
            )],
            &[&revenue_manager],
        )
        .await;

        let expected_tca_commission = tca_amount * tca_commission_bps as u64 / 10_000;
        let expected_tca_validator = tca_amount - expected_tca_commission;
        let expected_bca_commission = bca_amount * 500 / 10_000;
        let expected_bca_validator = bca_amount - expected_bca_commission;

        assert_eq!(
            self.lamports(&rakurai_bb).await - commission_before,
            expected_tca_commission + expected_bca_commission
        );
        assert_eq!(
            self.lamports(&validator).await - validator_before,
            expected_tca_validator + expected_bca_validator
        );

        assert!(
            self.fetch_revenue_share(tca)
                .await
                .ledger
                .entries
                .iter()
                .find(|e| e.epoch == creation_epoch)
                .unwrap()
                .claimed
        );
        assert!(
            self.fetch_revenue_share(bca)
                .await
                .ledger
                .entries
                .iter()
                .find(|e| e.epoch == creation_epoch)
                .unwrap()
                .claimed
        );
    }

    pub async fn phase7_close_revenue_shares(&mut self) {
        let rd_id = self.rd_id;
        let tca = self.tca;
        let bca = self.bca;
        let revenue_manager = clone_kp(&self.revenue_manager);
        let manager_pk = revenue_manager.pubkey();

        let manager_before = self.lamports(&manager_pk).await;

        self.process_ixs(
            &[close_revenue_share_account_ix(
                rd_id,
                CloseRevenueShareAccountAccounts {
                    revenue_share_account: tca,
                    manager_authority: manager_pk,
                },
            )],
            &[&revenue_manager],
        )
        .await;

        self.process_ixs(
            &[close_revenue_share_account_ix(
                rd_id,
                CloseRevenueShareAccountAccounts {
                    revenue_share_account: bca,
                    manager_authority: manager_pk,
                },
            )],
            &[&revenue_manager],
        )
        .await;

        assert!(self.account_missing(&tca).await);
        assert!(self.account_missing(&bca).await);
        assert!(
            self.lamports(&manager_pk).await > manager_before,
            "manager should reclaim rent from closed accounts"
        );
    }

    async fn lamports(&mut self, address: &Pubkey) -> u64 {
        self.context
            .banks_client
            .get_balance(*address)
            .await
            .unwrap()
    }

    async fn account_missing(&mut self, address: &Pubkey) -> bool {
        self.context
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .is_none()
    }

    async fn fetch_raa(&mut self) -> RakuraiActivationAccount {
        let account = self
            .context
            .banks_client
            .get_account(self.raa)
            .await
            .unwrap()
            .unwrap();
        let mut data = account.data.as_slice();
        RakuraiActivationAccount::try_deserialize(&mut data).unwrap()
    }

    async fn fetch_rd_config(&mut self) -> RewardDistributionConfigAccount {
        let account = self
            .context
            .banks_client
            .get_account(self.rd_config)
            .await
            .unwrap()
            .unwrap();
        let mut data = account.data.as_slice();
        RewardDistributionConfigAccount::try_deserialize(&mut data).unwrap()
    }

    async fn fetch_revenue_share(&mut self, address: Pubkey) -> RevenueShareAccount {
        let account = self
            .context
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap();
        let mut data = account.data.as_slice();
        RevenueShareAccount::try_deserialize(&mut data).unwrap()
    }

    pub async fn rca_lamports(&mut self) -> u64 {
        let rca = self.rca;
        self.lamports(&rca).await
    }
}

pub fn padded_name(label: &str) -> [u8; 32] {
    let mut name = [0u8; 32];
    let bytes = label.as_bytes();
    assert!(bytes.len() <= 32, "label too long");
    name[..bytes.len()].copy_from_slice(bytes);
    name
}
