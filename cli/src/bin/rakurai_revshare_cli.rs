use {
    anchor_lang::AccountDeserialize,
    clap::{Args, Parser, Subcommand, ValueEnum},
    colored::{ColoredString, Colorize},
    rakurai_cli::{
        normalize_to_url_if_moniker, parse_keypair, parse_pubkey, sign_and_send_instructions,
        sign_and_send_transaction,
    },
    reward_distribution::{
        sdk::{
            derive_revenue_share_account_address, derive_revenue_share_account_v1_address,
            instruction::{
                record_revenue_ix, record_revenue_v1_ix, settle_revenue_ix, RecordRevenueArgs,
                RecordRevenueShareAccounts, SettleRevenueAccounts, SettleRevenueArgs,
            },
        },
        state::{
            EpochAmountEntry, EpochAmountEntryV1, RevenueKind, RevenueShareAccount,
            RevenueShareAccountV1,
        },
    },
    solana_account_decoder_client_types::UiAccountEncoding,
    solana_rpc_client::rpc_client::RpcClient,
    solana_rpc_client_api::{
        config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        filter::{Memcmp, RpcFilterType},
    },
    solana_sdk::{
        account::Account, commitment_config::CommitmentConfig, instruction::Instruction,
        pubkey::Pubkey, rent::Rent, signature::Signer, system_instruction, system_program,
    },
    std::{error::Error, sync::Arc},
};

type CliResult<T = ()> = Result<T, Box<dyn Error>>;

/// Anchor account disc + field layout for revenue share headers.
const SHARE_KIND_OFFSET: usize = 8;
const NAME_OFFSET: usize = 9;
/// Default instructions per settle-all transaction (tx size limits).
const DEFAULT_TRANSFER_ALL_BATCH_SIZE: usize = 10;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Partner Tip and MevShare Revenue Settlement CLI for TCA/MCA vaults",
    arg_required_else_help = true,
    color = clap::ColorChoice::Always
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the Solana keypair used as transfer payer.
    #[arg(short, long, global = true, default_value = "~/.config/solana/id.json")]
    keypair: String,

    /// Solana RPC endpoint or moniker: m, t, d, l.
    #[arg(
        short,
        long,
        global = true,
        default_value = "t",
        value_parser = normalize_to_url_if_moniker
    )]
    url: String,

    /// Reward Distribution program ID.
    #[arg(
        short,
        long,
        required = true,
        value_parser = parse_pubkey,
        help = "Reward Distribution Program ID [testnet: A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB, mainnet-beta: RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB]"
    )]
    program_id: Pubkey,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch and display one TCA or MCA (requires vote pubkey).
    GetAccount(AccountArgs),
    /// List every TCA or MCA for a service (`--revenue-kind` + `--revenue-name`).
    GetAllAccounts(GetAllAccountsArgs),
    /// Show the unclaimed/unsettled record for one epoch.
    GetPendingRecord(PendingRecordArgs),
    /// Show every epoch record that still has an amount pending settlement.
    GetAllPendingRecords(AccountArgs),
    /// Record MCA MevShare revenue for the current epoch (post-pack partners).
    RecordRevenue(RecordRevenueCliArgs),
    /// Transfer SOL into a vault for one recorded epoch.
    Transfer(TransferArgs),
    /// Settle all pending epochs for every vault matching this service.
    TransferAll(TransferAllArgs),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ShareKindArg {
    Tip,
    #[value(name = "Mev-share", aliases = ["mev-share", "MevShare"])]
    MevShare,
}

impl From<ShareKindArg> for RevenueKind {
    fn from(value: ShareKindArg) -> Self {
        match value {
            ShareKindArg::Tip => RevenueKind::Tip,
            ShareKindArg::MevShare => RevenueKind::MevShare,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum AccountVersion {
    /// Prefer V1, then legacy. Fail only if neither exists.
    Auto,
    /// Legacy PDA only.
    Legacy,
    /// V1 PDA only.
    V1,
}

#[derive(Args, Clone)]
struct ServiceArgs {
    /// TCA (`Tip`) or MCA (`Mev-share`).
    #[arg(long = "revenue-kind", value_enum, required = true)]
    revenue_kind: ShareKindArg,

    /// Service revenue name (unique id assigned by Rakurai; PDA seed).
    #[arg(long = "revenue-name", required = true)]
    revenue_name: String,

    /// Select the PDA layout. Defaults to auto (include both layouts when listing).
    #[arg(
        long = "account-version",
        alias = "vault-version",
        value_enum,
        default_value = "auto"
    )]
    account_version: AccountVersion,
}

#[derive(Args, Clone)]
struct TargetArgs {
    #[command(flatten)]
    service: ServiceArgs,

    /// Validator vote account used in the PDA seeds.
    #[arg(short = 'v', long = "vote-pubkey", required = true, value_parser = parse_pubkey)]
    vote_pubkey: Pubkey,
}

#[derive(Args)]
struct AccountArgs {
    #[command(flatten)]
    target: TargetArgs,
}

#[derive(Args)]
struct PendingRecordArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Epoch ledger entry to inspect.
    #[arg(short, long)]
    epoch: u64,
}

#[derive(Args)]
struct RecordRevenueCliArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Lamports to add to the current-epoch recorded amount on the MCA.
    #[arg(short = 'x', long, required = true)]
    amount: u64,
}

#[derive(Args)]
struct TransferArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Epoch ledger entry being funded.
    #[arg(short, long)]
    epoch: u64,

    /// Lamports to transfer. Defaults to the full pending amount.
    #[arg(short = 'x', long)]
    amount: Option<u64>,

    /// Preview the transfer without sending a transaction.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Args)]
struct GetAllAccountsArgs {
    #[command(flatten)]
    service: ServiceArgs,

    /// After the summary table, print full per-account details.
    #[arg(long, default_value_t = false)]
    detail: bool,
}

#[derive(Args)]
struct TransferAllArgs {
    #[command(flatten)]
    service: ServiceArgs,

    /// Instructions (settle epochs) per transaction. Default: 10.
    #[arg(long, default_value_t = DEFAULT_TRANSFER_ALL_BATCH_SIZE)]
    batch_size: usize,

    /// Preview pending settlements without sending transactions.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

enum VaultAccount {
    Legacy {
        address: Pubkey,
        account: RevenueShareAccount,
        /// On-chain lamports at load time (for legacy pending vs balance).
        lamports: u64,
    },
    V1 {
        address: Pubkey,
        account: RevenueShareAccountV1,
        lamports: u64,
    },
}

impl VaultAccount {
    fn address(&self) -> Pubkey {
        match self {
            Self::Legacy { address, .. } | Self::V1 { address, .. } => *address,
        }
    }

    fn lamports(&self) -> u64 {
        match self {
            Self::Legacy { lamports, .. } | Self::V1 { lamports, .. } => *lamports,
        }
    }

    /// Claimable balance = total lamports minus rent-exempt minimum (matches on-chain claim).
    fn available_balance(&self) -> u64 {
        let space = match self {
            Self::Legacy { account, .. } => {
                RevenueShareAccount::space_for(account.max_epoch_entries as usize)
            }
            Self::V1 { account, .. } => {
                RevenueShareAccountV1::space_for(account.max_epoch_entries as usize)
            }
        };
        self.lamports()
            .saturating_sub(Rent::default().minimum_balance(space))
    }

    fn version_name(&self) -> &'static str {
        match self {
            Self::Legacy { .. } => "legacy",
            Self::V1 { .. } => "v1",
        }
    }

    fn record_authority(&self) -> Pubkey {
        match self {
            Self::Legacy { account, .. } => account.record_authority,
            Self::V1 { account, .. } => account.record_authority,
        }
    }

    fn share_kind(&self) -> RevenueKind {
        match self {
            Self::Legacy { account, .. } => account.share_kind,
            Self::V1 { account, .. } => account.share_kind,
        }
    }

    fn validator_vote(&self) -> Pubkey {
        match self {
            Self::Legacy { account, .. } => account.validator_vote,
            Self::V1 { account, .. } => account.validator_vote,
        }
    }

    fn revenue_name(&self) -> [u8; 32] {
        match self {
            Self::Legacy { account, .. } => account.name,
            Self::V1 { account, .. } => account.name,
        }
    }

    fn is_rakurai_tip_tca(&self) -> bool {
        match self {
            Self::Legacy { account, .. } => {
                account.share_kind == RevenueKind::Tip
                    && account.name == reward_distribution::state::RAKURAI_REVENUE_NAME
            }
            Self::V1 { account, .. } => account.is_rakurai_tip_tca(),
        }
    }

    /// Pending SOL still needed for partner settle (not manager claim).
    ///
    /// **Legacy:** Unclaimed epochs are covered by rent-aware vault balance (oldest first).
    /// Fully covered epochs are claim-only (not pending). When balance cannot cover
    /// recorded `amount`, pending = remaining shortfall.
    ///
    /// **V1:** Unclaimed with `transferred_amount < amount` → pending =
    /// `amount - transferred`. Fully transferred but unclaimed is claim-only.
    fn pending(&self, epoch: u64) -> CliResult<PendingAmount> {
        if let Some((_, p)) = self.all_pending().into_iter().find(|(e, _)| *e == epoch) {
            return Ok(p);
        }
        // No settle shortfall: report the raw entry (claim-only or missing settle need).
        match self {
            Self::Legacy { account, .. } => {
                let entry = find_legacy_entry(account, epoch)?;
                Ok(PendingAmount {
                    recorded: entry.amount,
                    transferred: None,
                    pending: 0,
                    claimed: entry.claimed,
                })
            }
            Self::V1 { account, .. } => {
                let entry = find_v1_entry(account, epoch)?;
                Ok(PendingAmount {
                    recorded: entry.amount,
                    transferred: Some(entry.transferred_amount),
                    pending: 0,
                    claimed: entry.claimed,
                })
            }
        }
    }

    fn all_pending(&self) -> Vec<(u64, PendingAmount)> {
        let mut records: Vec<(u64, PendingAmount)> = match self {
            Self::Legacy { account, .. } => {
                // Cover unclaimed epochs with available vault balance (oldest first).
                let mut cover = self.available_balance();
                let mut entries: Vec<_> = account
                    .ledger
                    .entries
                    .iter()
                    .filter(|e| !e.claimed && e.amount > 0)
                    .collect();
                entries.sort_unstable_by_key(|e| e.epoch);
                let mut out = Vec::new();
                for entry in entries {
                    if cover >= entry.amount {
                        // Fully funded → claim remains, not settle-pending.
                        cover -= entry.amount;
                        continue;
                    }
                    let pending = entry.amount - cover;
                    cover = 0;
                    out.push((
                        entry.epoch,
                        PendingAmount {
                            recorded: entry.amount,
                            transferred: None,
                            pending,
                            claimed: false,
                        },
                    ));
                }
                out
            }
            Self::V1 { account, .. } => {
                // Ledger transfer shortfall is settle-pending; fully transferred → claim only.
                account
                    .ledger
                    .entries
                    .iter()
                    .filter_map(|entry| {
                        if entry.claimed {
                            return None;
                        }
                        let pending = entry.amount.saturating_sub(entry.transferred_amount);
                        (pending > 0).then_some((
                            entry.epoch,
                            PendingAmount {
                                recorded: entry.amount,
                                transferred: Some(entry.transferred_amount),
                                pending,
                                claimed: false,
                            },
                        ))
                    })
                    .collect()
            }
        };
        records.sort_unstable_by_key(|(epoch, _)| *epoch);
        records
    }
}

struct PendingAmount {
    recorded: u64,
    transferred: Option<u64>,
    pending: u64,
    claimed: bool,
}

fn name_to_bytes(name: &str) -> Result<[u8; 32], String> {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return Err("name must not be empty".to_string());
    }
    if bytes.len() > 32 {
        return Err(format!(
            "name must be at most 32 bytes (got {})",
            bytes.len()
        ));
    }

    let mut result = [0u8; 32];
    result[..bytes.len()].copy_from_slice(bytes);
    Ok(result)
}

fn name_to_string(name: &[u8; 32]) -> String {
    let len = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    String::from_utf8_lossy(&name[..len]).into_owned()
}

fn find_legacy_entry(account: &RevenueShareAccount, epoch: u64) -> CliResult<&EpochAmountEntry> {
    account
        .ledger
        .entries
        .iter()
        .find(|entry| entry.epoch == epoch)
        .ok_or_else(|| format!("no legacy record found for epoch {epoch}").into())
}

fn find_v1_entry(account: &RevenueShareAccountV1, epoch: u64) -> CliResult<&EpochAmountEntryV1> {
    account
        .ledger
        .entries
        .iter()
        .find(|entry| entry.epoch == epoch)
        .ok_or_else(|| format!("no V1 record found for epoch {epoch}").into())
}

fn decode_legacy(
    address: Pubkey,
    raw: Option<Account>,
    program_id: Pubkey,
) -> CliResult<Option<VaultAccount>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.owner != program_id {
        return Err(format!(
            "legacy account {address} is owned by {}, expected {program_id}",
            raw.owner
        )
        .into());
    }

    let lamports = raw.lamports;
    let mut data = raw.data.as_slice();
    match RevenueShareAccount::try_deserialize(&mut data) {
        Ok(account) => Ok(Some(VaultAccount::Legacy {
            address,
            account,
            lamports,
        })),
        // Wrong layout (e.g. V1 PDA data / unrelated program account): treat as missing.
        Err(_) => Ok(None),
    }
}

fn decode_v1(
    address: Pubkey,
    raw: Option<Account>,
    program_id: Pubkey,
) -> CliResult<Option<VaultAccount>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    if raw.owner != program_id {
        return Err(format!(
            "V1 account {address} is owned by {}, expected {program_id}",
            raw.owner
        )
        .into());
    }

    let lamports = raw.lamports;
    let mut data = raw.data.as_slice();
    match RevenueShareAccountV1::try_deserialize(&mut data) {
        Ok(account) => Ok(Some(VaultAccount::V1 {
            address,
            account,
            lamports,
        })),
        // Wrong layout (e.g. legacy PDA data): treat as missing.
        Err(_) => Ok(None),
    }
}

fn decode_for_version(
    address: Pubkey,
    raw: Account,
    program_id: Pubkey,
    account_version: AccountVersion,
) -> CliResult<Option<VaultAccount>> {
    if raw.owner != program_id {
        return Ok(None);
    }
    Ok(match account_version {
        AccountVersion::Legacy => decode_legacy(address, Some(raw), program_id)?,
        AccountVersion::V1 => decode_v1(address, Some(raw), program_id)?,
        // Prefer V1 when both would fail-safe; try V1 first then legacy.
        AccountVersion::Auto => decode_v1(address, Some(raw.clone()), program_id)?
            .or(decode_legacy(address, Some(raw), program_id)?),
    })
}

fn load_target(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    target: &TargetArgs,
) -> CliResult<VaultAccount> {
    let name = name_to_bytes(&target.service.revenue_name)?;
    let kind = target.service.revenue_kind.into();
    let legacy_address =
        derive_revenue_share_account_address(&program_id, kind, &name, &target.vote_pubkey).0;
    let v1_address =
        derive_revenue_share_account_v1_address(&program_id, kind, &name, &target.vote_pubkey).0;
    let mut accounts = rpc_client.get_multiple_accounts(&[legacy_address, v1_address])?;
    let v1 = decode_v1(
        v1_address,
        accounts
            .pop()
            .ok_or("RPC omitted the V1 account response")?,
        program_id,
    )?;
    let legacy = decode_legacy(
        legacy_address,
        accounts
            .pop()
            .ok_or("RPC omitted the legacy account response")?,
        program_id,
    )?;

    match target.service.account_version {
        AccountVersion::Legacy => legacy
            .ok_or_else(|| format!("legacy account does not exist at {legacy_address}").into()),
        AccountVersion::V1 => {
            v1.ok_or_else(|| format!("V1 account does not exist at {v1_address}").into())
        }
        AccountVersion::Auto => match (v1, legacy) {
            (Some(account), _) => Ok(account),
            (None, Some(account)) => Ok(account),
            (None, None) => Err(format!(
                "no V1 or legacy account exists (V1 {v1_address}, legacy {legacy_address})"
            )
            .into()),
        },
    }
}

fn share_kind_byte(kind: RevenueKind) -> u8 {
    match kind {
        RevenueKind::Tip => 0,
        RevenueKind::MevShare => 1,
    }
}

fn load_service_accounts(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    service: &ServiceArgs,
) -> CliResult<Vec<VaultAccount>> {
    let name = name_to_bytes(&service.revenue_name)?;
    let kind: RevenueKind = service.revenue_kind.into();
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            SHARE_KIND_OFFSET,
            vec![share_kind_byte(kind)],
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(NAME_OFFSET, name.to_vec())),
    ];

    let accounts = rpc_client.get_program_accounts_with_config(
        &program_id,
        RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                data_slice: None,
                commitment: Some(CommitmentConfig::confirmed()),
                min_context_slot: None,
            },
            with_context: None,
            sort_results: None,
        },
    )?;

    let mut vaults = Vec::new();
    for (address, raw) in accounts {
        if let Some(vault) = decode_for_version(address, raw, program_id, service.account_version)?
        {
            if vault.share_kind() == kind && vault.revenue_name() == name {
                vaults.push(vault);
            }
        }
    }

    vaults.sort_by_key(|vault| (vault.validator_vote().to_string(), vault.version_name()));
    Ok(vaults)
}

fn print_heading(title: &str) {
    println!("📌 {}", title.bold().underline().blue());
}

fn print_field(icon: ColoredString, label: &str, value: impl std::fmt::Display) {
    // Match rakurai-activation: icon + fixed-width label + value.
    println!("   {} {:<12} {}", icon, label, value);
}

fn display_account(vault: &VaultAccount, balance: u64) {
    let (kind, name, vote, record_authority) = match vault {
        VaultAccount::Legacy { account, .. } => (
            account.share_kind,
            &account.name,
            account.validator_vote,
            account.record_authority,
        ),
        VaultAccount::V1 { account, .. } => (
            account.share_kind,
            &account.name,
            account.validator_vote,
            account.record_authority,
        ),
    };

    print_heading("Rakurai Reward Distribution Account");
    print_field(
        "🔗".cyan(),
        "Pubkey:",
        vault.address().to_string().bold().green(),
    );
    print_field("📦".cyan(), "Account:", vault.version_name().blue());
    print_field("📝".cyan(), "Type:", kind_name(kind).magenta());
    print_field("📝".cyan(), "Name:", name_to_string(name).magenta());
    print_field("🔑".red(), "Vote:", vote.to_string());
    print_field("🔏".magenta(), "Record auth:", record_authority.to_string());
    print_field(
        "💰".green(),
        "Balance:",
        format_total_with_sol(balance).yellow(),
    );
}

fn display_pending(vault: &VaultAccount, epoch: u64, pending: &PendingAmount) {
    print_heading("Pending Revenue Record");
    print_field(
        "🔗".cyan(),
        "Pubkey:",
        vault.address().to_string().bold().green(),
    );
    print_field("📦".cyan(), "Account:", vault.version_name().blue());
    display_pending_amount(epoch, pending);
}

fn display_pending_amount(epoch: u64, pending: &PendingAmount) {
    print_field("🕒".cyan(), "Epoch:", epoch.to_string().blue());
    print_field(
        "📝".cyan(),
        "Recorded:",
        format_total_with_sol(pending.recorded).yellow(),
    );
    if let Some(transferred) = pending.transferred {
        print_field(
            "💰".green(),
            "Transferred:",
            format_total_with_sol(transferred).yellow(),
        );
    } else {
        print_field(
            "💰".green(),
            "Funded (via balance):",
            format_total_with_sol(pending.recorded.saturating_sub(pending.pending)).yellow(),
        );
    }
    print_field(
        "💰".yellow(),
        "Pending settle:",
        format_total_with_sol(pending.pending).yellow(),
    );
    print_field(
        if pending.claimed {
            "✅".green()
        } else {
            "❌".red()
        },
        "Claimed:",
        pending.claimed.to_string().blue(),
    );
}

fn kind_name(kind: RevenueKind) -> &'static str {
    match kind {
        RevenueKind::Tip => "Tip",
        RevenueKind::MevShare => "Mev-share",
    }
}

/// Abbreviate a pubkey (or similar base58 string) as `ABCDEF....UVWXYZ`.
fn short_pubkey(s: &str) -> String {
    if s.len() <= 12 {
        return s.to_string();
    }
    format!("{}....{}", &s[..6], &s[s.len() - 6..])
}

/// SOL display decimals (1 SOL = 1e9 lamports).
const SOL_DECIMALS: usize = 5;

fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Format an amount as integer lamports, or `-` when zero.
fn format_lamports(lamports: u64) -> String {
    if lamports == 0 {
        "-".to_string()
    } else {
        lamports.to_string()
    }
}

/// Format as `X.XXXXX SOL` only (footer totals). Zero → `-`.
fn format_sol_only(lamports: u64) -> String {
    if lamports == 0 {
        return "-".to_string();
    }
    format!(
        "{sol:.prec$} SOL",
        sol = lamports_to_sol(lamports),
        prec = SOL_DECIMALS
    )
}

/// Format as `N (X.XXXXX SOL)`. Zero → `-`.
fn format_row_total_with_sol(lamports: u64) -> String {
    if lamports == 0 {
        return "-".to_string();
    }
    format!(
        "{lamports} ({sol:.prec$} SOL)",
        lamports = lamports,
        sol = lamports_to_sol(lamports),
        prec = SOL_DECIMALS
    )
}

/// Format as `N lamports (X.XXXXX SOL)`.
fn format_total_with_sol(lamports: u64) -> String {
    if lamports == 0 {
        return format!("0 lamports ({z:.prec$} SOL)", z = 0.0, prec = SOL_DECIMALS);
    }
    format!(
        "{lamports} lamports ({sol:.prec$} SOL)",
        lamports = lamports,
        sol = lamports_to_sol(lamports),
        prec = SOL_DECIMALS
    )
}

type PendingByVote = std::collections::BTreeMap<String, std::collections::BTreeMap<u64, u64>>;

/// Aggregate unclaimed pending amounts from vaults into vote → epoch → lamports.
/// Skips Rakurai tip TCAs when `skip_rakurai_tip` is true (transfer paths).
fn aggregate_pending_by_vote(
    vaults: &[VaultAccount],
    skip_rakurai_tip: bool,
) -> (PendingByVote, Vec<u64>, u64) {
    let mut by_vote: PendingByVote = std::collections::BTreeMap::new();
    let mut epochs = std::collections::BTreeSet::new();
    for vault in vaults {
        if skip_rakurai_tip && vault.is_rakurai_tip_tca() {
            continue;
        }
        let vote = vault.validator_vote().to_string();
        let vote_map = by_vote.entry(vote).or_default();
        for (epoch, pending) in vault.all_pending() {
            if pending.pending == 0 {
                continue;
            }
            epochs.insert(epoch);
            *vote_map.entry(epoch).or_default() += pending.pending;
        }
    }
    let epochs: Vec<u64> = epochs.into_iter().collect();
    let mut grand_total = 0u64;
    for vote_map in by_vote.values() {
        for amount in vote_map.values() {
            grand_total = grand_total.saturating_add(*amount);
        }
    }
    // Drop validators with no remaining pending after filters.
    by_vote.retain(|_, m| !m.is_empty());
    (by_vote, epochs, grand_total)
}

/// Print summary + pivot table (epochs = columns, vote rows, TOTAL col with SOL).
fn print_pending_pivot_table(by_vote: &PendingByVote, epochs: &[u64], grand_total: u64) {
    print_field(
        "💰".green(),
        "Total:",
        format_total_with_sol(grand_total).bold().yellow(),
    );
    print_field(
        "🔑".red(),
        "Validators:",
        by_vote.len().to_string().magenta(),
    );
    print_field("🕒".cyan(), "Epochs:", epochs.len().to_string().magenta());

    println!();
    if epochs.is_empty() {
        println!("   {}", "(no pending revenue records)".yellow());
        return;
    }

    let mut epoch_totals = vec![0u64; epochs.len()];
    let mut row_totals: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for (vote, vote_map) in by_vote {
        let mut row_total = 0u64;
        for (i, epoch) in epochs.iter().enumerate() {
            let amount = vote_map.get(epoch).copied().unwrap_or(0);
            epoch_totals[i] = epoch_totals[i].saturating_add(amount);
            row_total = row_total.saturating_add(amount);
        }
        row_totals.insert(vote.clone(), row_total);
    }

    // Wide enough for `N (X.XXXXX SOL)` row totals; body epoch cells stay lamports-only.
    const VOTE_W: usize = 15;
    const EPOCH_W: usize = 15;
    const TOTAL_W: usize = 30;

    print!("   {:<width$}", "Vote".bold(), width = VOTE_W);
    for epoch in epochs {
        print!("  {:>width$}", epoch.to_string().bold(), width = EPOCH_W);
    }
    print!(
        "  {:>width$}",
        "TOTAL (lamports / SOL)".bold(),
        width = TOTAL_W
    );
    println!();
    let line_w = VOTE_W + epochs.len() * (EPOCH_W + 2) + TOTAL_W + 2;
    println!("   {}", "-".repeat(line_w));

    for (vote, vote_map) in by_vote {
        print!("   {:<width$}", short_pubkey(vote), width = VOTE_W);
        for epoch in epochs {
            let amount = vote_map.get(epoch).copied().unwrap_or(0);
            print!("  {:>width$}", format_lamports(amount), width = EPOCH_W);
        }
        let row_total = row_totals.get(vote).copied().unwrap_or(0);
        print!(
            "  {:>width$}",
            format_row_total_with_sol(row_total),
            width = TOTAL_W
        );
        println!();
    }

    println!("   {}", "-".repeat(line_w));
    // Footer TOTAL: SOL only (5 dp).
    print!("   {:<width$}", "TOTAL".bold(), width = VOTE_W);
    for total in &epoch_totals {
        print!(
            "  {:>width$}",
            format_sol_only(*total).bold().yellow(),
            width = EPOCH_W
        );
    }
    print!(
        "  {:>width$}",
        format_sol_only(grand_total).bold().yellow(),
        width = TOTAL_W
    );
    println!();
}

fn process_get_account(rpc_client: &RpcClient, program_id: Pubkey, args: AccountArgs) -> CliResult {
    let vault = load_target(rpc_client, program_id, &args.target)?;
    let balance = rpc_client.get_balance(&vault.address())?;
    display_account(&vault, balance);
    Ok(())
}

fn process_get_all_accounts(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    args: GetAllAccountsArgs,
) -> CliResult {
    let vaults = load_service_accounts(rpc_client, program_id, &args.service)?;
    let kind: RevenueKind = args.service.revenue_kind.into();

    print_heading("Service Revenue Accounts");
    print_field("📝".cyan(), "Type:", kind_name(kind).magenta());
    print_field("📝".cyan(), "Name:", args.service.revenue_name.magenta());
    print_field("📦".cyan(), "Accounts:", vaults.len().to_string().magenta());

    if vaults.is_empty() {
        println!(
            "\n   {}",
            "No TCA/MCA found for this revenue-kind + revenue-name.".yellow()
        );
        return Ok(());
    }

    let (by_vote, epochs, grand_total) = aggregate_pending_by_vote(&vaults, false);
    print_pending_pivot_table(&by_vote, &epochs, grand_total);

    if !args.detail {
        println!(
            "\n   {}",
            "--detail for per-account pubkey, layout, record authority, and balance.".dimmed()
        );
        return Ok(());
    }

    print_heading("Account details");
    for vault in &vaults {
        let balance = rpc_client.get_balance(&vault.address())?;
        let pending = vault.all_pending();
        let pending_total: u64 = pending.iter().map(|(_, p)| p.pending).sum();

        println!();
        print_field(
            "🔗".cyan(),
            "Pubkey:",
            vault.address().to_string().bold().green(),
        );
        print_field("📦".cyan(), "Account:", vault.version_name().blue());
        print_field("🔑".red(), "Vote:", vault.validator_vote().to_string());
        print_field(
            "🔏".magenta(),
            "Record auth:",
            vault.record_authority().to_string(),
        );
        print_field(
            "💰".green(),
            "Balance:",
            format_total_with_sol(balance).yellow(),
        );
        print_field(
            "🕒".cyan(),
            "Pending:",
            format!(
                "{} epoch(s), {} lamports",
                pending.len().to_string().blue(),
                pending_total.to_string().yellow()
            ),
        );
        for (epoch, amount) in &pending {
            println!(
                "      epoch {:>8}: {}",
                epoch.to_string().blue(),
                format_total_with_sol(amount.pending).yellow()
            );
        }
    }
    Ok(())
}

fn process_get_pending(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    args: PendingRecordArgs,
) -> CliResult {
    let vault = load_target(rpc_client, program_id, &args.target)?;
    let pending = vault.pending(args.epoch)?;
    display_pending(&vault, args.epoch, &pending);
    Ok(())
}

fn process_get_all_pending(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    args: AccountArgs,
) -> CliResult {
    let vault = load_target(rpc_client, program_id, &args.target)?;
    let records = vault.all_pending();

    print_heading("Pending Revenue Records");
    print_field(
        "🔗".cyan(),
        "Pubkey:",
        vault.address().to_string().bold().green(),
    );
    print_field("📦".cyan(), "Account:", vault.version_name().blue());
    print_field("📝".cyan(), "Records:", records.len().to_string().magenta());
    for (epoch, pending) in records {
        println!();
        display_pending_amount(epoch, &pending);
    }
    Ok(())
}

fn process_record_revenue(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: RecordRevenueCliArgs,
) -> CliResult {
    if args.target.service.revenue_kind != ShareKindArg::MevShare {
        return Err(
            "record-revenue is for MCA post-pack partners only; use --revenue-kind Mev-share. \
             TCA tip amounts are recorded by the validator during leader turns."
                .into(),
        );
    }
    if args.amount == 0 {
        return Err("record amount must be greater than zero".into());
    }

    let vault = load_target(&rpc_client, program_id, &args.target)?;
    if vault.share_kind() != RevenueKind::MevShare {
        return Err("loaded vault is not an MCA (Mev-share); refusing record-revenue".into());
    }

    let authority = parse_keypair(keypair_path)?;
    let expected = vault.record_authority();
    if authority.pubkey() != expected {
        return Err(format!(
            "keypair {} is not the MCA record_authority {}; use the authority assigned at MCA init",
            authority.pubkey(),
            expected
        )
        .into());
    }

    let accounts = RecordRevenueShareAccounts {
        revenue_share_account: vault.address(),
        record_authority: authority.pubkey(),
    };
    let instruction = match &vault {
        VaultAccount::Legacy { .. } => record_revenue_ix(
            program_id,
            RecordRevenueArgs {
                amount: args.amount,
            },
            accounts,
        ),
        VaultAccount::V1 { .. } => record_revenue_v1_ix(
            program_id,
            RecordRevenueArgs {
                amount: args.amount,
            },
            accounts,
        ),
    };

    let clock_epoch = rpc_client.get_epoch_info()?.epoch;

    print_heading("Partner MevShare Record Revenue");
    print_field(
        "🔗".cyan(),
        "Vault:",
        vault.address().to_string().bold().green(),
    );
    print_field("📦".cyan(), "Account:", vault.version_name().blue());
    print_field(
        "🕒".cyan(),
        "Epoch:",
        format!("{clock_epoch} (current cluster epoch)").blue(),
    );
    print_field(
        "💰".green(),
        "Amount:",
        format_total_with_sol(args.amount).yellow(),
    );
    print_field("🔏".magenta(), "Signer:", authority.pubkey().to_string());
    sign_and_send_transaction(rpc_client, instruction, &authority)
}

fn settle_instruction(
    program_id: Pubkey,
    vault: &VaultAccount,
    epoch: u64,
    amount: u64,
    payer: Pubkey,
) -> CliResult<Instruction> {
    if vault.is_rakurai_tip_tca() {
        return Err(
            "Rakurai tip TCA records transfers automatically; settle is not allowed for that vault"
                .into(),
        );
    }

    match vault {
        VaultAccount::Legacy { address, .. } => {
            Ok(system_instruction::transfer(&payer, address, amount))
        }
        VaultAccount::V1 { address, .. } => Ok(settle_revenue_ix(
            program_id,
            SettleRevenueArgs { epoch, amount },
            SettleRevenueAccounts {
                revenue_share_account: *address,
                payer,
                system_program: system_program::ID,
            },
        )),
    }
}

fn process_transfer(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: TransferArgs,
) -> CliResult {
    let vault = load_target(&rpc_client, program_id, &args.target)?;
    let pending = vault.pending(args.epoch)?;
    if pending.claimed {
        return Err(format!("epoch {} is already claimed", args.epoch).into());
    }

    let amount = args.amount.unwrap_or(pending.pending);
    if amount == 0 {
        return Err(format!("epoch {} has no pending amount to transfer", args.epoch).into());
    }
    if amount > pending.pending {
        return Err(format!(
            "transfer amount {amount} exceeds pending amount {}; refusing overpayment",
            pending.pending
        )
        .into());
    }

    let payer = parse_keypair(keypair_path)?;
    let instruction = settle_instruction(program_id, &vault, args.epoch, amount, payer.pubkey())?;

    print_heading("Partner Tip / MevShare Settlement Transfer");
    print_field(
        "🔗".cyan(),
        "Vault:",
        vault.address().to_string().bold().green(),
    );
    print_field("📦".cyan(), "Account:", vault.version_name().blue());
    print_field(
        "🔑".red(),
        "Vote:",
        short_pubkey(&vault.validator_vote().to_string()),
    );
    print_field("🕒".cyan(), "Epoch:", args.epoch.to_string().blue());
    print_field(
        "💰".green(),
        "Amount:",
        format_total_with_sol(amount).yellow(),
    );
    print_field("🔑".red(), "Payer:", payer.pubkey().to_string());
    if args.dry_run {
        println!(
            "\n   {}",
            "Dry run only — no transaction was sent.".yellow()
        );
        return Ok(());
    }
    sign_and_send_transaction(rpc_client, instruction, &payer)
}

struct PendingSettlement {
    vote: Pubkey,
    version: &'static str,
    epoch: u64,
    amount: u64,
    instruction: Instruction,
}

fn process_transfer_all(
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    keypair_path: &str,
    args: TransferAllArgs,
) -> CliResult {
    if args.batch_size == 0 {
        return Err("--batch-size must be at least 1".into());
    }

    let vaults = load_service_accounts(&rpc_client, program_id, &args.service)?;
    let payer = parse_keypair(keypair_path)?;
    let kind: RevenueKind = args.service.revenue_kind.into();

    let mut jobs = Vec::new();
    for vault in &vaults {
        if vault.is_rakurai_tip_tca() {
            continue;
        }
        for (epoch, pending) in vault.all_pending() {
            if pending.pending == 0 {
                continue;
            }
            let instruction =
                settle_instruction(program_id, vault, epoch, pending.pending, payer.pubkey())?;
            jobs.push(PendingSettlement {
                vote: vault.validator_vote(),
                version: vault.version_name(),
                epoch,
                amount: pending.pending,
                instruction,
            });
        }
    }

    print_heading("Transfer All Pending Settlements");
    print_field("📝".cyan(), "Type:", kind_name(kind).magenta());
    print_field("📝".cyan(), "Name:", args.service.revenue_name.magenta());
    print_field("📦".cyan(), "Vaults:", vaults.len().to_string().magenta());
    print_field(
        "🕒".cyan(),
        "Settlements:",
        jobs.len().to_string().magenta(),
    );
    print_field(
        "📦".cyan(),
        "Ix/txn:",
        args.batch_size.to_string().magenta(),
    );
    print_field("🔑".red(), "Payer:", payer.pubkey().to_string());

    let (by_vote, epochs, grand_total) = aggregate_pending_by_vote(&vaults, true);
    print_pending_pivot_table(&by_vote, &epochs, grand_total);

    if jobs.is_empty() {
        println!("\n   {}", "Nothing pending to settle.".green());
        return Ok(());
    }

    if args.dry_run {
        println!(
            "\n   {}",
            "Dry run only — no transactions were sent.".yellow()
        );
        return Ok(());
    }

    println!();
    let mut sent = 0usize;
    for chunk in jobs.chunks(args.batch_size) {
        let instructions: Vec<_> = chunk.iter().map(|job| job.instruction.clone()).collect();
        print_heading(&format!(
            "Sending batch of {} settlement(s)",
            instructions.len()
        ));
        // Preview this batch briefly (vote + epoch).
        for job in chunk {
            println!(
                "   {}  {}  epoch {:>6}  {} lamports",
                short_pubkey(&job.vote.to_string()).cyan(),
                job.version.blue(),
                job.epoch.to_string().blue(),
                job.amount.to_string().yellow()
            );
        }
        sign_and_send_instructions(rpc_client.clone(), &instructions, &payer)?;
        sent += instructions.len();
    }

    println!(
        "\n   {} {}",
        "✅".green(),
        format!("Settled {sent} pending epoch(s) across all matching vaults.").green()
    );
    Ok(())
}

fn main() -> CliResult {
    let cli = Cli::parse();
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        cli.url,
        CommitmentConfig::confirmed(),
    ));

    match cli.command {
        Commands::GetAccount(args) => process_get_account(&rpc_client, cli.program_id, args),
        Commands::GetAllAccounts(args) => {
            process_get_all_accounts(&rpc_client, cli.program_id, args)
        }
        Commands::GetPendingRecord(args) => process_get_pending(&rpc_client, cli.program_id, args),
        Commands::GetAllPendingRecords(args) => {
            process_get_all_pending(&rpc_client, cli.program_id, args)
        }
        Commands::RecordRevenue(args) => {
            process_record_revenue(rpc_client, cli.program_id, &cli.keypair, args)
        }
        Commands::Transfer(args) => {
            process_transfer(rpc_client, cli.program_id, &cli.keypair, args)
        }
        Commands::TransferAll(args) => {
            process_transfer_all(rpc_client, cli.program_id, &cli.keypair, args)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_zero_padded() {
        let name = name_to_bytes("rakurai").unwrap();
        assert_eq!(&name[..7], b"rakurai");
        assert!(name[7..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn rejects_invalid_name_lengths() {
        assert!(name_to_bytes("").is_err());
        assert!(name_to_bytes(&"x".repeat(33)).is_err());
    }
}
