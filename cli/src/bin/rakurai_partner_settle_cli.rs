use {
    anchor_lang::{prelude::Pubkey as AnchorPubkey, AccountDeserialize},
    clap::{Args, Parser, Subcommand, ValueEnum},
    colored::{ColoredString, Colorize},
    rakurai_cli::{
        normalize_to_url_if_moniker, parse_keypair, parse_pubkey, sign_and_send_transaction,
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
    solana_commitment_config::CommitmentConfig,
    solana_instruction::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::RpcClient,
    solana_signer::Signer,
    solana_system_interface::{instruction as system_instruction, program as system_program},
    std::{error::Error, sync::Arc},
};

type CliResult<T = ()> = Result<T, Box<dyn Error>>;

fn to_anchor(pubkey: Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(pubkey.as_array().clone())
}

fn from_anchor(pubkey: AnchorPubkey) -> Pubkey {
    Pubkey::new_from_array(pubkey.to_bytes())
}

fn to_solana_instruction(
    mut ix: anchor_lang::solana_program::instruction::Instruction,
) -> Instruction {
    let acct_metas: Vec<AccountMeta> = ix
        .accounts
        .iter_mut()
        .map(|acct| AccountMeta {
            pubkey: Pubkey::new_from_array(acct.pubkey.to_bytes().clone()),
            is_signer: acct.is_signer,
            is_writable: acct.is_writable,
        })
        .collect();
    Instruction::new_with_bytes(
        Pubkey::new_from_array(ix.program_id.to_bytes()),
        &ix.data,
        acct_metas,
    )
}

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
    /// Fetch and display a TCA or MCA.
    GetAccount(AccountArgs),
    /// Show the unclaimed/unsettled record for one epoch.
    GetPendingRecord(PendingRecordArgs),
    /// Show every epoch record that still has an amount pending settlement.
    GetAllPendingRecords(AccountArgs),
    /// Record MCA MevShare revenue for the current epoch (post-pack partners).
    RecordRevenue(RecordRevenueCliArgs),
    /// Transfer SOL into a vault for one recorded epoch.
    Transfer(TransferArgs),
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
struct TargetArgs {
    /// TCA (`Tip`) or MCA (`Mev-share`).
    #[arg(long = "revenue-kind", value_enum, required = true)]
    revenue_kind: ShareKindArg,

    /// Service revenue name (unique id assigned by Rakurai; PDA seed).
    #[arg(long = "revenue-name", required = true)]
    revenue_name: String,

    /// Validator vote account used in the PDA seeds.
    #[arg(short = 'v', long = "vote-pubkey", required = true, value_parser = parse_pubkey)]
    vote_pubkey: Pubkey,

    /// Select the PDA layout. Defaults to auto (prefer V1, fall back to legacy).
    #[arg(
        long = "account-version",
        alias = "vault-version",
        value_enum,
        default_value = "auto"
    )]
    account_version: AccountVersion,
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
}

enum VaultAccount {
    Legacy {
        address: Pubkey,
        account: RevenueShareAccount,
    },
    V1 {
        address: Pubkey,
        account: RevenueShareAccountV1,
    },
}

impl VaultAccount {
    fn address(&self) -> Pubkey {
        match self {
            Self::Legacy { address, .. } | Self::V1 { address, .. } => *address,
        }
    }

    fn version_name(&self) -> &'static str {
        match self {
            Self::Legacy { .. } => "legacy",
            Self::V1 { .. } => "v1",
        }
    }

    fn record_authority(&self) -> Pubkey {
        match self {
            Self::Legacy { account, .. } => {
                Pubkey::new_from_array(*account.record_authority.as_array())
            }
            Self::V1 { account, .. } => {
                Pubkey::new_from_array(*account.record_authority.as_array())
            }
        }
    }

    fn share_kind(&self) -> RevenueKind {
        match self {
            Self::Legacy { account, .. } => account.share_kind,
            Self::V1 { account, .. } => account.share_kind,
        }
    }

    fn pending(&self, epoch: u64) -> CliResult<PendingAmount> {
        match self {
            Self::Legacy { account, .. } => {
                let entry = find_legacy_entry(account, epoch)?;
                Ok(PendingAmount {
                    recorded: entry.amount,
                    transferred: None,
                    pending: if entry.claimed { 0 } else { entry.amount },
                    claimed: entry.claimed,
                })
            }
            Self::V1 { account, .. } => {
                let entry = find_v1_entry(account, epoch)?;
                Ok(PendingAmount {
                    recorded: entry.amount,
                    transferred: Some(entry.transferred_amount),
                    pending: if entry.claimed {
                        0
                    } else {
                        entry.amount.saturating_sub(entry.transferred_amount)
                    },
                    claimed: entry.claimed,
                })
            }
        }
    }

    fn all_pending(&self) -> Vec<(u64, PendingAmount)> {
        let mut records: Vec<_> = match self {
            Self::Legacy { account, .. } => account
                .ledger
                .entries
                .iter()
                .filter(|entry| !entry.claimed && entry.amount > 0)
                .map(|entry| {
                    (
                        entry.epoch,
                        PendingAmount {
                            recorded: entry.amount,
                            transferred: None,
                            pending: entry.amount,
                            claimed: false,
                        },
                    )
                })
                .collect(),
            Self::V1 { account, .. } => account
                .ledger
                .entries
                .iter()
                .filter_map(|entry| {
                    let pending = entry.amount.saturating_sub(entry.transferred_amount);
                    (!entry.claimed && pending > 0).then_some((
                        entry.epoch,
                        PendingAmount {
                            recorded: entry.amount,
                            transferred: Some(entry.transferred_amount),
                            pending,
                            claimed: false,
                        },
                    ))
                })
                .collect(),
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
    raw: Option<(Pubkey, Vec<u8>)>,
    program_id: Pubkey,
) -> CliResult<Option<VaultAccount>> {
    let Some((owner, data)) = raw else {
        return Ok(None);
    };
    if owner != program_id {
        return Err(
            format!("legacy account {address} is owned by {owner}, expected {program_id}").into(),
        );
    }

    let mut data = data.as_slice();
    let account = RevenueShareAccount::try_deserialize(&mut data)
        .map_err(|error| format!("failed to decode legacy account {address}: {error}"))?;
    Ok(Some(VaultAccount::Legacy { address, account }))
}

fn decode_v1(
    address: Pubkey,
    raw: Option<(Pubkey, Vec<u8>)>,
    program_id: Pubkey,
) -> CliResult<Option<VaultAccount>> {
    let Some((owner, data)) = raw else {
        return Ok(None);
    };
    if owner != program_id {
        return Err(
            format!("V1 account {address} is owned by {owner}, expected {program_id}").into(),
        );
    }

    let mut data = data.as_slice();
    let account = RevenueShareAccountV1::try_deserialize(&mut data)
        .map_err(|error| format!("failed to decode V1 account {address}: {error}"))?;
    Ok(Some(VaultAccount::V1 { address, account }))
}

fn load_target(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    target: &TargetArgs,
) -> CliResult<VaultAccount> {
    let name = name_to_bytes(&target.revenue_name)?;
    let kind = target.revenue_kind.into();
    let program_id_anchor = to_anchor(program_id);
    let vote_pubkey = to_anchor(target.vote_pubkey);
    let legacy_address = from_anchor(
        derive_revenue_share_account_address(&program_id_anchor, kind, &name, &vote_pubkey).0,
    );
    let v1_address = from_anchor(
        derive_revenue_share_account_v1_address(&program_id_anchor, kind, &name, &vote_pubkey).0,
    );
    let mut accounts = rpc_client.get_multiple_accounts(&[legacy_address, v1_address])?;
    let v1 = decode_v1(
        v1_address,
        accounts
            .pop()
            .ok_or("RPC omitted the V1 account response")?
            .map(|account| (account.owner, account.data)),
        program_id,
    )?;
    let legacy = decode_legacy(
        legacy_address,
        accounts
            .pop()
            .ok_or("RPC omitted the legacy account response")?
            .map(|account| (account.owner, account.data)),
        program_id,
    )?;

    match target.account_version {
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
        format!(
            "{} lamports ({:.9} SOL)",
            balance.to_string().yellow(),
            balance as f64 / 1_000_000_000.0
        ),
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
        format!("{} lamports", pending.recorded.to_string().yellow()),
    );
    if let Some(transferred) = pending.transferred {
        print_field(
            "💰".green(),
            "Transferred:",
            format!("{} lamports", transferred.to_string().yellow()),
        );
    } else {
        print_field(
            "💰".green(),
            "Transferred:",
            "Unavailable (legacy layout)".yellow(),
        );
    }
    print_field(
        "💰".yellow(),
        "Pending:",
        format!("{} lamports", pending.pending.to_string().yellow()),
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

fn process_get_account(rpc_client: &RpcClient, program_id: Pubkey, args: AccountArgs) -> CliResult {
    let vault = load_target(rpc_client, program_id, &args.target)?;
    let balance = rpc_client.get_balance(&vault.address())?;
    display_account(&vault, balance);
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
    if args.target.revenue_kind != ShareKindArg::MevShare {
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
        revenue_share_account: to_anchor(vault.address()),
        record_authority: to_anchor(authority.pubkey()),
    };
    let program_id_anchor = to_anchor(program_id);
    let instruction = match &vault {
        VaultAccount::Legacy { .. } => to_solana_instruction(record_revenue_ix(
            program_id_anchor,
            RecordRevenueArgs {
                amount: args.amount,
            },
            accounts,
        )),
        VaultAccount::V1 { .. } => to_solana_instruction(record_revenue_v1_ix(
            program_id_anchor,
            RecordRevenueArgs {
                amount: args.amount,
            },
            accounts,
        )),
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
        format!(
            "{} lamports ({:.9} SOL)",
            args.amount.to_string().yellow(),
            args.amount as f64 / 1_000_000_000.0
        ),
    );
    print_field("🔏".magenta(), "Signer:", authority.pubkey().to_string());
    sign_and_send_transaction(rpc_client, instruction, &authority)
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
    let instruction = match &vault {
        VaultAccount::Legacy { address, .. } => {
            system_instruction::transfer(&payer.pubkey(), address, amount)
        }
        VaultAccount::V1 { address, account } => {
            if account.share_kind == RevenueKind::Tip
                && account.name == reward_distribution::state::RAKURAI_REVENUE_NAME
            {
                return Err(
                    "Rakurai tip TCAV1 records transfers automatically; settle_revenue is not allowed"
                        .into(),
                );
            }
            let program_id = to_anchor(program_id);
            to_solana_instruction(settle_revenue_ix(
                program_id,
                SettleRevenueArgs {
                    epoch: args.epoch,
                    amount,
                },
                SettleRevenueAccounts {
                    revenue_share_account: to_anchor(*address),
                    payer: to_anchor(payer.pubkey()),
                    system_program: to_anchor(system_program::id()),
                },
            ))
        }
    };

    print_heading("Partner Tip / MevShare Settlement Transfer");
    print_field(
        "🔗".cyan(),
        "Vault:",
        vault.address().to_string().bold().green(),
    );
    print_field("📦".cyan(), "Account:", vault.version_name().blue());
    print_field("🕒".cyan(), "Epoch:", args.epoch.to_string().blue());
    print_field(
        "💰".green(),
        "Amount:",
        format!(
            "{} lamports ({:.9} SOL)",
            amount.to_string().yellow(),
            amount as f64 / 1_000_000_000.0
        ),
    );
    print_field("🔑".red(), "Payer:", payer.pubkey().to_string());
    sign_and_send_transaction(rpc_client, instruction, &payer)
}

fn main() -> CliResult {
    let cli = Cli::parse();
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        cli.url,
        CommitmentConfig::confirmed(),
    ));

    match cli.command {
        Commands::GetAccount(args) => process_get_account(&rpc_client, cli.program_id, args),
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
