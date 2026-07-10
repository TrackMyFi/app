//! SimpleFIN Bridge integration: automatic daily import of account balance
//! snapshots and transactions.
//!
//! Flow: the user pastes a one-time SETUP TOKEN (a base64-encoded claim URL)
//! from bridge.simplefin.org. Claiming it (a single POST) yields the permanent
//! ACCESS URL — a URL with embedded credentials — which is stored in the OS
//! keychain, never in the database. Every sync GETs `{access}/accounts` and
//! imports data for each SimpleFIN account the user has linked to a local
//! account (`account.simplefin_id`).
//!
//! Rate limits: SimpleFIN allows up to 24 requests per day, but only refreshes
//! from institutions about once every 24h (at unpredictable times per bank).
//! The scheduler in `lib.rs` syncs 3h after the last success — often enough to
//! pick up their daily refresh promptly, at most 8 requests/day — retrying no
//! sooner than 6h after a failed attempt, and only while the app window is
//! focused. "Sync now" is always allowed (user action).
//!
//! Gaps: bank connections drop and can stay broken for days or weeks. Every
//! fetch therefore starts its transaction window at the last SUCCESSFUL sync
//! (minus a small overlap), not "yesterday" — so when a connection comes back,
//! the whole gap is backfilled in one pull. Duplicates from overlapping
//! windows are impossible: every imported transaction carries its SimpleFIN id
//! under a unique index and is inserted with INSERT OR IGNORE.
//!
//! Provenance flags for debugging: imported transactions get
//! `import_source = 'simplefin'` plus their `simplefin_id`; imported balance
//! snapshots get `account_balance.source = 'simplefin'`.

use crate::db::Db;
use base64::Engine as _;
use chrono::{DateTime, Local, TimeZone, Utc};
use libsql::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;
use ts_rs::TS;

const KEYCHAIN_USER: &str = "simplefin-access-url";

/// Sync 3h after the last successful sync. SimpleFIN only refreshes upstream
/// data ~once a day at an unknown time, so this bounds how stale we can be
/// after their refresh lands while staying well under their 24-requests/day
/// limit (≤8/day, focused-only).
const SYNC_AFTER_SUCCESS_HOURS: i64 = 3;
/// After a failed attempt, wait this long before retrying automatically.
const RETRY_AFTER_FAILURE_HOURS: i64 = 6;
/// Re-fetch this many days before the last success so nothing on the boundary
/// is missed; the unique simplefin_id index absorbs the duplicates.
const OVERLAP_DAYS: i64 = 3;
/// Transaction lookback for the very first sync after connecting.
const FIRST_SYNC_LOOKBACK_DAYS: i64 = 90;
/// Extra margin behind the oldest cached pending transaction when it anchors
/// the fetch window (banks can post a charge backdated to — rarely, just
/// before — its transacted date).
const PENDING_ANCHOR_BUFFER_DAYS: i64 = 1;
/// Oldest allowed start of a user-requested custom range. SimpleFIN bridges
/// serve roughly a year of history at most, and a year also bounds the
/// response size.
const CUSTOM_RANGE_MAX_DAYS: i64 = 365;
/// How often the background scheduler re-checks whether a sync is due.
pub const SCHEDULER_TICK_SECS: u64 = 1800; // 30 minutes
/// ± window (days) for matching the two sides of a transfer across accounts.
/// Mirrors TRANSFER_DATE_TOLERANCE_DAYS in src/lib/csv/mapping.ts.
const TRANSFER_DATE_TOLERANCE_DAYS: f64 = 3.0;
/// ± window (days) for matching a SimpleFIN transaction against a manual/CSV
/// one in the duplicate review — SimpleFIN's posted date can lag a manually
/// entered purchase date by a day or two for card transactions. A judgment
/// call (it happens to equal OVERLAP_DAYS, but isn't derived from it).
const DUPLICATE_DATE_TOLERANCE_DAYS: f64 = 3.0;

/// Managed state: serializes concurrent syncs (scheduler tick vs. manual click).
pub struct SimpleFinShared {
    pub lock: AsyncMutex<()>,
}

impl SimpleFinShared {
    pub fn new() -> Self {
        Self { lock: AsyncMutex::new(()) }
    }
}

// ---- keychain (access URL contains credentials — treated like a password) ----

#[cfg(target_os = "macos")]
fn access_url_get() -> Result<Option<String>, String> {
    crate::sync::macos_keychain::get(crate::sync::KEYCHAIN_SERVICE, KEYCHAIN_USER)
}
#[cfg(target_os = "macos")]
fn access_url_set(url: &str) -> Result<(), String> {
    crate::sync::macos_keychain::set(crate::sync::KEYCHAIN_SERVICE, KEYCHAIN_USER, url)
}
#[cfg(target_os = "macos")]
fn access_url_delete() -> Result<(), String> {
    crate::sync::macos_keychain::delete(crate::sync::KEYCHAIN_SERVICE, KEYCHAIN_USER)
}

#[cfg(not(target_os = "macos"))]
fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(crate::sync::KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| e.to_string())
}
#[cfg(not(target_os = "macos"))]
fn access_url_get() -> Result<Option<String>, String> {
    match keyring_entry()?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
#[cfg(not(target_os = "macos"))]
fn access_url_set(url: &str) -> Result<(), String> {
    keyring_entry()?.set_password(url).map_err(|e| e.to_string())
}
#[cfg(not(target_os = "macos"))]
fn access_url_delete() -> Result<(), String> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

// ---- SimpleFIN protocol types ----

/// SimpleFIN sends money values as strings ("1234.56") per spec, but be
/// tolerant of servers that send bare numbers.
#[derive(Deserialize)]
#[serde(untagged)]
enum NumOrStr {
    N(f64),
    S(String),
}

impl NumOrStr {
    fn as_f64(&self) -> Option<f64> {
        match self {
            NumOrStr::N(n) => Some(*n),
            NumOrStr::S(s) => s.trim().parse().ok(),
        }
    }
}

#[derive(Deserialize)]
struct SfinAccountSet {
    #[serde(default)]
    errors: Vec<String>,
    #[serde(default)]
    accounts: Vec<SfinAccount>,
}

#[derive(Deserialize)]
struct SfinOrg {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    domain: Option<String>,
}

#[derive(Deserialize)]
struct SfinAccount {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    org: Option<SfinOrg>,
    /// Either a currency code string or a custom-currency URL/object.
    #[serde(default)]
    currency: Option<serde_json::Value>,
    balance: NumOrStr,
    #[serde(rename = "balance-date")]
    balance_date: i64,
    #[serde(default)]
    transactions: Vec<SfinTxn>,
}

#[derive(Deserialize)]
struct SfinTxn {
    id: String,
    #[serde(default)]
    posted: i64,
    #[serde(default)]
    transacted_at: Option<i64>,
    amount: NumOrStr,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    payee: Option<String>,
    #[serde(default)]
    memo: Option<String>,
    #[serde(default)]
    pending: Option<bool>,
    /// Vendor-supplied category. Not part of the core spec but sent by some
    /// institutions/bridges either top-level or inside `extra`.
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    extra: Option<serde_json::Value>,
}

// ---- types exported to the frontend ----

#[derive(Serialize, Deserialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SimpleFinRemoteAccount {
    pub id: String,
    pub name: String,
    pub org: Option<String>,
    /// Balance exactly as the bank reports it (liabilities usually negative).
    pub balance: f64,
    /// "YYYY-MM-DD" of the bank's balance-date.
    pub balance_date: String,
    pub currency: Option<String>,
    /// Local account this SimpleFIN account is linked to, if any. Recomputed
    /// from the `account` table on every read (never trusted from the cache).
    pub linked_account_id: Option<i32>,
}

#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SimpleFinStatus {
    pub connected: bool,
    pub claimed_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    /// Messages from the bridge's `errors` field — typically "Connection to
    /// <bank> may need attention". These arrive even on successful syncs.
    pub bridge_errors: Vec<String>,
    /// Remote accounts seen on the last fetch (cached; no extra API call).
    pub accounts: Vec<SimpleFinRemoteAccount>,
}

#[derive(Serialize, Clone, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SimpleFinSyncSummary {
    #[ts(type = "number")]
    pub accounts_synced: i64,
    #[ts(type = "number")]
    pub transactions_added: i64,
    #[ts(type = "number")]
    pub snapshots_added: i64,
    /// Cross-account income/expense pairs collapsed into canonical transfers,
    /// plus counterpart rows absorbed into user-converted transfers.
    #[ts(type = "number")]
    pub transfers_detected: i64,
    /// Whether the pending-transaction cache changed this sync (the set is
    /// wiped and re-inserted every time; this compares the row counts).
    pub pending_changed: bool,
    pub bridge_errors: Vec<String>,
}

/// Payload of the `simplefin-syncing` event: `syncing: true` when a sync
/// starts, `syncing: false` plus the outcome when it finishes — lets the
/// frontend surface progress/result notifications even for background syncs.
#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SimpleFinSyncingEvent {
    pub syncing: bool,
    pub error: Option<String>,
    #[ts(type = "number")]
    pub transactions_added: i64,
    #[ts(type = "number")]
    pub snapshots_added: i64,
}

/// A transaction still pending at the bank, cached for awareness display only.
/// Never enters `txn` and never counts toward any total — pending rows mutate
/// or vanish (even their SimpleFIN id can change once posted), so the cache is
/// wiped and re-inserted on every sync.
#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SimpleFinPendingTransaction {
    pub id: i32,
    pub account_id: i32,
    pub amount: f64,
    pub description: String,
    /// "YYYY-MM-DD" (transacted date when the bank sends one, else posted).
    pub date: String,
    /// "income" | "expense" (sign-derived, same as the posted import path).
    pub txn_type: String,
    pub simplefin_id: String,
    /// Every distinct description field the bank sent, unedited.
    pub raw_description: Option<String>,
}

/// One candidate duplicate pair for the post-import review: a SimpleFIN-imported
/// transaction and a non-SimpleFIN one (manual/CSV/paycheck) on the same account
/// that look like the same real-world event.
#[derive(Serialize, Clone, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SimpleFinDuplicateCandidate {
    pub account_id: i32,
    pub account_name: String,
    pub amount: f64,
    /// "income" | "expense" (transfers are excluded from matching).
    pub txn_type: String,
    pub simplefin_txn_id: i32,
    pub simplefin_date: String,
    pub simplefin_description: String,
    pub other_txn_id: i32,
    pub other_date: String,
    pub other_description: String,
    pub other_import_source: String,
    /// Resolution bucket: "ordinary" (delete the non-SimpleFIN row, keep its
    /// snapshot), "net_deposit" (paycheck-linked deposit — higher stakes,
    /// user opts in), or "contribution" (paycheck-linked row on an investment
    /// account — resolution is reversed: delete the SimpleFIN row, which can
    /// never carry is_contribution/paycheck_id).
    pub bucket: String,
}

// ---- small helpers ----

fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
}

/// Unix seconds → local calendar date "YYYY-MM-DD" (matches how the app
/// stores transaction dates and balance recorded_at values).
fn ts_to_date(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

/// Pull a vendor category out of a SimpleFIN transaction, checking the
/// top-level `category` field first, then common shapes inside `extra`.
fn vendor_category(t: &SfinTxn) -> Option<String> {
    if let Some(c) = &t.category {
        let c = c.trim();
        if !c.is_empty() {
            return Some(c.to_string());
        }
    }
    let extra = t.extra.as_ref()?;
    for key in ["category", "categories", "vendor_category"] {
        match extra.get(key) {
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => {
                return Some(s.trim().to_string());
            }
            Some(serde_json::Value::Array(items)) => {
                let joined = items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" / ");
                if !joined.is_empty() {
                    return Some(joined);
                }
            }
            _ => {}
        }
    }
    None
}

/// Payment-processor wrappers that obscure the real merchant: "LINK.COM*
/// SIMPLEFIN BR ..." is a SimpleFIN charge routed through Link, not a
/// purchase from link.com. Matched case-insensitively at the start of a
/// field; an optional "*" after the prefix is stripped too.
const PROCESSOR_PREFIXES: &[&str] = &["LINK.COM"];

/// Strip a processor wrapper from one candidate field. Returns None when the
/// field is JUST the processor name (e.g. a payee of "Link.com") — such a
/// field says nothing about the purchase and must not win the precedence.
fn strip_processor_prefix(field: &str) -> Option<String> {
    for prefix in PROCESSOR_PREFIXES {
        if field.len() >= prefix.len() && field[..prefix.len()].eq_ignore_ascii_case(prefix) {
            let rest = field[prefix.len()..].trim_start_matches('*').trim();
            return if rest.is_empty() { None } else { Some(rest.to_string()) };
        }
    }
    Some(field.to_string())
}

/// Pick the display description (payee → description → memo, processor
/// wrappers stripped) and assemble the raw description: every distinct
/// non-empty field the bank sent, joined unedited — kept on the row so the
/// user can always see the original text behind any cleanup.
fn select_description(t: &SfinTxn) -> (String, Option<String>) {
    let fields: Vec<&str> = [t.payee.as_deref(), t.description.as_deref(), t.memo.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut raw_parts: Vec<&str> = Vec::new();
    for f in &fields {
        if !raw_parts.iter().any(|p| p.eq_ignore_ascii_case(f)) {
            raw_parts.push(f);
        }
    }
    let raw = if raw_parts.is_empty() { None } else { Some(raw_parts.join(" · ")) };

    let display = fields
        .iter()
        .find_map(|f| strip_processor_prefix(f))
        .unwrap_or_else(|| "SimpleFIN transaction".to_string());
    (display, raw)
}

/// Local calendar date "YYYY-MM-DD" → unix seconds at local midnight
/// (inverse of `ts_to_date`, same local-time convention).
fn date_to_ts(d: &str) -> Option<i64> {
    let nd = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()?;
    Local
        .from_local_datetime(&nd.and_hms_opt(0, 0, 0)?)
        .single()
        .map(|dt| dt.timestamp())
}

/// Start of the fetch window for a regular sync: OVERLAP_DAYS before the last
/// success (or the first-sync lookback), extended further back whenever a
/// cached pending transaction is older. A charge can take longer than the
/// overlap to post (e.g. over a holiday weekend), and banks often backdate
/// the posted date to the original transaction date — once the rolling window
/// slides past that date the bridge stops returning the transaction entirely,
/// so it would vanish from the pending cache without ever being imported.
/// Anchoring to the oldest pending row keeps every transaction the app has
/// shown as pending inside the window until it actually posts.
fn window_start(
    now: DateTime<Utc>,
    last_success: Option<DateTime<Utc>>,
    oldest_pending_date: Option<&str>,
) -> i64 {
    let base = match last_success {
        Some(t) => t.timestamp() - OVERLAP_DAYS * 86_400,
        None => now.timestamp() - FIRST_SYNC_LOOKBACK_DAYS * 86_400,
    };
    match oldest_pending_date
        .and_then(date_to_ts)
        .map(|ts| ts - PENDING_ANCHOR_BUFFER_DAYS * 86_400)
    {
        Some(anchor) => base.min(anchor),
        None => base,
    }
}

/// Validate a user-requested custom range and convert it to the fetch
/// window's unix timestamps: (start of the start day, start of the day AFTER
/// the end day — SimpleFIN's end-date is exclusive, so this makes the chosen
/// end date inclusive).
fn parse_custom_range(
    start: &str,
    end: &str,
    today: chrono::NaiveDate,
) -> Result<(i64, i64), String> {
    let parse = |s: &str| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| "Enter dates as YYYY-MM-DD.".to_string())
    };
    let start_d = parse(start)?;
    let end_d = parse(end)?;
    if start_d > end_d {
        return Err("The start date must be on or before the end date.".to_string());
    }
    if end_d > today {
        return Err("The end date can't be in the future.".to_string());
    }
    if (today - start_d).num_days() > CUSTOM_RANGE_MAX_DAYS {
        return Err(format!(
            "SimpleFIN provides about a year of history — pick a start date within the last \
             {CUSTOM_RANGE_MAX_DAYS} days."
        ));
    }
    let to_ts = |d: chrono::NaiveDate| {
        date_to_ts(&d.format("%Y-%m-%d").to_string())
            .ok_or_else(|| "Enter dates as YYYY-MM-DD.".to_string())
    };
    Ok((to_ts(start_d)?, to_ts(end_d + chrono::Duration::days(1))?))
}

/// Whether an automatic sync is due. Pure so it's testable: 3h after the last
/// success, but never within 6h of the last attempt (failed attempts back off
/// instead of hammering a broken connection).
fn sync_due(
    now: DateTime<Utc>,
    last_success: Option<DateTime<Utc>>,
    last_attempt: Option<DateTime<Utc>>,
) -> bool {
    let success_due = match last_success {
        Some(t) => now - t >= chrono::Duration::hours(SYNC_AFTER_SUCCESS_HOURS),
        None => true,
    };
    // The backoff applies only to FAILED attempts. `last_attempt_at` is
    // written when a sync starts and `last_success_at` when it finishes, so
    // an attempt strictly newer than the last success is one that failed —
    // a successful sync must not push the next auto-sync past the normal
    // interval.
    let last_failed_attempt = match (last_attempt, last_success) {
        (Some(a), Some(s)) => (a > s).then_some(a),
        (a, None) => a,
        (None, Some(_)) => None,
    };
    let attempt_ok = match last_failed_attempt {
        Some(t) => now - t >= chrono::Duration::hours(RETRY_AFTER_FAILURE_HOURS),
        None => true,
    };
    success_due && attempt_ok
}

/// Lock/busy errors that clear on their own once the holder finishes — retry
/// material, unlike corruption or constraint failures.
fn is_transient_lock_error(e: &str) -> bool {
    let e = e.to_lowercase();
    e.contains("locked") || e.contains("busy")
}

/// First matching category rule wins; matching is a case-insensitive substring
/// test on the description — mirrors `applyMapping` in src/lib/csv/mapping.ts.
fn match_category(rules: &[(String, String)], description: &str) -> String {
    let desc = description.to_lowercase();
    rules
        .iter()
        .find(|(keyword, _)| desc.contains(&keyword.to_lowercase()))
        .map(|(_, category)| category.clone())
        .unwrap_or_else(|| "uncategorized".to_string())
}

// ---- HTTP client ----

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())
}

/// Decode a setup token into its claim URL. Tokens are base64-encoded URLs;
/// accept a raw URL too (some tools hand you the claim URL directly).
fn decode_setup_token(token: &str) -> Result<String, String> {
    let t = token.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        return Ok(t.to_string());
    }
    let cleaned: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| "That doesn't look like a SimpleFIN setup token.".to_string())?;
    let url = String::from_utf8(bytes)
        .map_err(|_| "That doesn't look like a SimpleFIN setup token.".to_string())?;
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(url)
    } else {
        Err("That doesn't look like a SimpleFIN setup token.".to_string())
    }
}

/// Claim a setup token (single-use POST) → permanent access URL.
async fn claim_access_url(setup_token: &str) -> Result<String, String> {
    let claim_url = decode_setup_token(setup_token)?;
    let resp = http()?
        .post(&claim_url)
        .header(reqwest::header::CONTENT_LENGTH, "0")
        .send()
        .await
        .map_err(|e| format!("Could not reach SimpleFIN: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Claiming the setup token failed (HTTP {}). Setup tokens are single-use — \
             generate a fresh one at your SimpleFIN Bridge account and try again.",
            resp.status().as_u16()
        ));
    }
    let access_url = resp.text().await.map_err(|e| e.to_string())?.trim().to_string();
    if access_url.starts_with("http://") || access_url.starts_with("https://") {
        Ok(access_url)
    } else {
        Err("SimpleFIN returned an unexpected response while claiming the token.".to_string())
    }
}

/// GET `{access}/accounts`, moving the URL-embedded credentials into a proper
/// Basic auth header (hyper does not transmit userinfo from the URL).
/// Returns the parsed set plus the raw response body (for the debug dump —
/// the body carries no credentials, those live in the request).
async fn fetch_accounts(
    access_url: &str,
    start_date: Option<i64>,
    end_date: Option<i64>,
    include_pending: bool,
) -> Result<(SfinAccountSet, String), String> {
    let mut url =
        reqwest::Url::parse(access_url).map_err(|_| "Stored access URL is invalid.".to_string())?;
    let user = url.username().to_string();
    let pass = url.password().map(str::to_string);
    let _ = url.set_username("");
    let _ = url.set_password(None);
    let path = format!("{}/accounts", url.path().trim_end_matches('/'));
    url.set_path(&path);
    if let Some(ts) = start_date {
        url.query_pairs_mut().append_pair("start-date", &ts.to_string());
    }
    if let Some(ts) = end_date {
        url.query_pairs_mut().append_pair("end-date", &ts.to_string());
    }
    // Without this the bridge returns posted transactions only — pending ones
    // are opt-in per the SimpleFIN protocol. Custom-range backfills skip it:
    // they must not disturb the pending cache (see import_account_set).
    if include_pending {
        url.query_pairs_mut().append_pair("pending", "1");
    }

    let mut req = http()?.get(url);
    if !user.is_empty() {
        req = req.basic_auth(user, pass);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Could not reach SimpleFIN: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(
            "SimpleFIN rejected this app's access (HTTP 403). The access token may have been \
             revoked — disconnect and connect again with a new setup token."
                .to_string(),
        );
    }
    if !status.is_success() {
        return Err(format!("SimpleFIN returned HTTP {}.", status.as_u16()));
    }
    let raw = resp
        .text()
        .await
        .map_err(|e| format!("Could not read the SimpleFIN response: {e}"))?;
    let set = serde_json::from_str::<SfinAccountSet>(&raw)
        .map_err(|e| format!("Could not parse the SimpleFIN response: {e}"))?;
    Ok((set, raw))
}

/// Dump the raw `/accounts` response to `simplefin-last-response.json` in the
/// app data dir, pretty-printed when possible. Debugging aid: shows exactly
/// what the bridge sent (pending flags, extra fields, vendor categories).
/// Overwritten every sync so it never grows; contains account/transaction
/// data but no credentials — same sensitivity and same directory as the
/// database file itself, and deliberately NOT in the DB so it can never ride
/// along with Turso cloud sync.
fn dump_raw_response(app: &AppHandle, raw: &str) {
    let Ok(dir) = crate::sync::data_dir(app) else { return };
    let pretty = serde_json::from_str::<serde_json::Value>(raw)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or_else(|_| raw.to_string());
    let body = format!("// fetched {}\n{pretty}\n", now_iso());
    let _ = std::fs::write(dir.join("simplefin-last-response.json"), body);
}

// ---- sync state (simplefin_state singleton row) ----

#[derive(Default)]
struct SfState {
    claimed_at: Option<String>,
    last_attempt_at: Option<String>,
    last_success_at: Option<String>,
    last_error: Option<String>,
    bridge_errors: Option<String>,
    accounts_json: Option<String>,
}

async fn read_state(conn: &Connection) -> Result<SfState, String> {
    let mut rows = conn
        .query(
            "SELECT claimed_at, last_attempt_at, last_success_at, last_error, bridge_errors, \
             accounts_json FROM simplefin_state WHERE id = 1",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    match rows.next().await.map_err(|e| e.to_string())? {
        Some(r) => Ok(SfState {
            claimed_at: r.get(0).map_err(|e| e.to_string())?,
            last_attempt_at: r.get(1).map_err(|e| e.to_string())?,
            last_success_at: r.get(2).map_err(|e| e.to_string())?,
            last_error: r.get(3).map_err(|e| e.to_string())?,
            bridge_errors: r.get(4).map_err(|e| e.to_string())?,
            accounts_json: r.get(5).map_err(|e| e.to_string())?,
        }),
        None => Ok(SfState::default()),
    }
}

/// UPDATE one nullable column of the singleton state row. `column` must come
/// from the fixed set below — it is interpolated into the SQL.
async fn set_state(conn: &Connection, column: &str, value: Option<&str>) -> Result<(), String> {
    debug_assert!([
        "claimed_at",
        "last_attempt_at",
        "last_success_at",
        "last_error",
        "bridge_errors",
        "accounts_json",
    ]
    .contains(&column));
    // The row is seeded by the migration, but a fresh cloud DB adopted from
    // another device could race — make sure it exists.
    conn.execute("INSERT OR IGNORE INTO simplefin_state (id) VALUES (1)", ())
        .await
        .map_err(|e| e.to_string())?;
    conn.execute(
        &format!("UPDATE simplefin_state SET {column} = ?1 WHERE id = 1"),
        params![value.map(str::to_string)],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---- import ----

struct LinkedAccount {
    account_id: i32,
    is_liability: bool,
}

/// Import an account set into the DB: one balance snapshot per linked account
/// (deduped per bank-reported date) and every non-pending transaction (deduped
/// by SimpleFIN id). Returns the summary plus the remote-account cache entries.
///
/// `rebuild_pending` is true for regular syncs (the response carries the full
/// current pending set, so the cache is wiped and re-inserted) and false for
/// custom-range backfills (fetched without `pending=1`, covering an arbitrary
/// window — wiping would throw away perfectly current pending rows).
async fn import_account_set(
    conn: &Connection,
    set: &SfinAccountSet,
    rebuild_pending: bool,
) -> Result<(SimpleFinSyncSummary, Vec<SimpleFinRemoteAccount>), String> {
    // simplefin_id → local account (id + liability flag, for balance sign).
    // Both lookup cursors are scoped so their statements are closed before the
    // write loop below — an open reader on this connection makes the writes'
    // WAL auto-checkpoint fail ("database table is locked").
    let mut linked: std::collections::HashMap<String, LinkedAccount> =
        std::collections::HashMap::new();
    {
        let mut rows = conn
            .query(
                "SELECT simplefin_id, id, type FROM account WHERE simplefin_id IS NOT NULL",
                (),
            )
            .await
            .map_err(|e| e.to_string())?;
        while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
            let sfin_id: String = r.get(0).map_err(|e| e.to_string())?;
            let account_id: i32 = r.get(1).map_err(|e| e.to_string())?;
            let ty: String = r.get(2).map_err(|e| e.to_string())?;
            linked.insert(
                sfin_id,
                LinkedAccount { account_id, is_liability: ty == "liability" || ty == "mortgage" },
            );
        }
    }

    // Category rules, first-match-wins in id order (same as the CSV importer).
    let mut rules: Vec<(String, String)> = Vec::new();
    {
        let mut rule_rows = conn
            .query("SELECT keyword, category FROM category_rules ORDER BY id", ())
            .await
            .map_err(|e| e.to_string())?;
        while let Some(r) = rule_rows.next().await.map_err(|e| e.to_string())? {
            rules.push((
                r.get::<String>(0).map_err(|e| e.to_string())?,
                r.get::<String>(1).map_err(|e| e.to_string())?,
            ));
        }
    }

    let mut summary = SimpleFinSyncSummary::default();
    summary.bridge_errors = set.errors.clone();
    let mut cache: Vec<SimpleFinRemoteAccount> = Vec::new();
    // Accounts whose snapshot chain may need re-anchoring, with the earliest
    // date this sync touched per account.
    let mut reproject: Vec<(i32, String)> = Vec::new();
    let created_at = now_iso();

    // The pending cache is rebuilt from scratch every regular sync: pending
    // rows at the bank mutate or vanish, so the previous set is worthless.
    // Rows are re-inserted below as each linked account's transactions are
    // walked.
    let pending_wiped = if rebuild_pending {
        conn.execute("DELETE FROM simplefin_pending_txn", ())
            .await
            .map_err(|e| e.to_string())? as i64
    } else {
        0
    };
    let mut pending_inserted = 0i64;

    for acct in &set.accounts {
        let raw_balance = acct.balance.as_f64().unwrap_or(0.0);
        let balance_date = ts_to_date(acct.balance_date);
        cache.push(SimpleFinRemoteAccount {
            id: acct.id.clone(),
            name: acct.name.clone().unwrap_or_else(|| acct.id.clone()),
            org: acct.org.as_ref().and_then(|o| o.name.clone().or_else(|| o.domain.clone())),
            balance: raw_balance,
            balance_date: balance_date.clone(),
            currency: acct.currency.as_ref().and_then(|c| c.as_str().map(str::to_string)),
            linked_account_id: None, // filled in at read time
        });

        let Some(link) = linked.get(&acct.id) else { continue };
        summary.accounts_synced += 1;
        let mut min_touched: Option<String> = None;

        // Balance snapshot. Banks report liabilities as negative amounts; the
        // app stores liability balances as positive debt (see side_delta).
        let balance = if link.is_liability { -raw_balance } else { raw_balance };
        // Read the existing snapshot into locals and DROP the cursor before any
        // write. A `Rows` still alive on this connection holds its read
        // statement open, and the write's WAL auto-checkpoint then fails with
        // "Failed to checkpoint WAL: database table is locked" (same rule as
        // reproject_account: never write while a query is streaming).
        let existing: Option<(i32, f64)> = {
            let mut rows = conn
                .query(
                    "SELECT id, balance FROM account_balance \
                     WHERE account_id = ?1 AND source = 'simplefin' AND recorded_at = ?2 \
                     ORDER BY id DESC LIMIT 1",
                    params![link.account_id, balance_date.clone()],
                )
                .await
                .map_err(|e| e.to_string())?;
            match rows.next().await.map_err(|e| e.to_string())? {
                Some(r) => Some((
                    r.get(0).map_err(|e| e.to_string())?,
                    r.get(1).map_err(|e| e.to_string())?,
                )),
                None => None,
            }
        };
        match existing {
            Some((snap_id, old)) => {
                // Same-day re-report with a different value: update in place
                // rather than piling up snapshots for one day.
                if (old - balance).abs() > 0.005 {
                    conn.execute(
                        "UPDATE account_balance SET balance = ?1 WHERE id = ?2",
                        params![balance, snap_id],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    min_touched = Some(balance_date.clone());
                }
            }
            None => {
                conn.execute(
                    "INSERT INTO account_balance (account_id, balance, recorded_at, source) \
                     VALUES (?1, ?2, ?3, 'simplefin')",
                    params![link.account_id, balance, balance_date.clone()],
                )
                .await
                .map_err(|e| e.to_string())?;
                summary.snapshots_added += 1;
                min_touched = Some(balance_date.clone());
            }
        }

        // Transactions. Pending ones never enter `txn` — they mutate or
        // vanish, and the next daily sync picks them up once posted — but they
        // ARE cached in simplefin_pending_txn for awareness display.
        for t in &acct.transactions {
            let Some(signed) = t.amount.as_f64() else { continue };
            let ts = if t.posted > 0 { t.posted } else { t.transacted_at.unwrap_or(0) };
            if ts <= 0 {
                continue;
            }
            let date = ts_to_date(ts);
            let (ty, amount) =
                if signed < 0.0 { ("expense", -signed) } else { ("income", signed) };
            let (description, raw_description) = select_description(t);

            if t.pending == Some(true) {
                // The NOT EXISTS guards skip a pending row whose id already
                // posted into the ledger (some bridges keep the id stable) or
                // was consumed by a transfer collapse — either way it would
                // read as a double entry.
                pending_inserted += conn
                    .execute(
                        "INSERT INTO simplefin_pending_txn (account_id, simplefin_id, \
                         amount, description, date, type, created_at, raw_description) \
                         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8 \
                         WHERE NOT EXISTS (SELECT 1 FROM txn \
                           WHERE simplefin_id = ?2 OR simplefin_counterpart_id = ?2)",
                        params![
                            link.account_id,
                            t.id.clone(),
                            amount,
                            description,
                            date,
                            ty,
                            created_at.clone(),
                            raw_description
                        ],
                    )
                    .await
                    .map_err(|e| e.to_string())? as i64;
                continue;
            }
            let category = match_category(&rules, &description);
            let vendor_cat = vendor_category(t);

            // No generated balance snapshots for synced transactions — the
            // bridge's own balance snapshot is the authoritative anchor.
            // The NOT EXISTS guard skips ids consumed by a transfer collapse:
            // the deleted counterpart row's id is no longer in the unique
            // simplefin_id index, so OR IGNORE alone would re-import it from
            // the overlap window.
            let inserted = conn
                .execute(
                    "INSERT OR IGNORE INTO txn (account_id, transfer_account_id, amount, \
                     description, date, type, category, is_contribution, is_withdrawal, \
                     import_source, generated_balance_id, generated_balance_to_id, \
                     vendor_category, simplefin_id, created_at, updated_at, raw_description) \
                     SELECT ?1, NULL, ?2, ?3, ?4, ?5, ?6, 0, 0, 'simplefin', NULL, NULL, \
                     ?7, ?8, ?9, ?9, ?10 \
                     WHERE NOT EXISTS \
                     (SELECT 1 FROM txn WHERE simplefin_counterpart_id = ?8)",
                    params![
                        link.account_id,
                        amount,
                        description,
                        date,
                        ty,
                        category,
                        vendor_cat,
                        t.id.clone(),
                        created_at.clone(),
                        raw_description
                    ],
                )
                .await
                .map_err(|e| e.to_string())?;
            summary.transactions_added += inserted as i64;
        }

        if let Some(date) = min_touched {
            reproject.push((link.account_id, date));
        }
    }

    // A simplefin snapshot acts as a manual anchor in the balance chain; if it
    // landed before existing transaction-generated snapshots, ripple forward.
    for (account_id, from_date) in reproject {
        crate::commands::transactions::reproject_accounts(conn, &[Some(account_id)], &from_date)
            .await?;
    }

    // Suppress noise BEFORE transfer collapse: a suppressed fee/gain row must
    // never be mistaken for one side of a same-amount transfer pair.
    crate::commands::suppress_rules::apply_suppress_rules(conn).await?;

    // Absorb before collapse: a user's hand-converted transfer is deliberate,
    // so it claims its counterpart ahead of the heuristic pairing pass.
    summary.transfers_detected = absorb_into_user_transfers(conn).await?;
    summary.transfers_detected += collapse_transfer_pairs(conn).await?;

    // Count comparison only — a same-count content mutation slips through, but
    // it self-corrects on the next page visit and the next sync.
    summary.pending_changed = pending_wiped != pending_inserted;

    // A backfill can import the posted form of a row still sitting in the
    // (untouched) pending cache — drop such rows so the UI doesn't show the
    // same purchase twice until the next regular sync rebuilds the cache.
    if !rebuild_pending {
        let cleared = conn
            .execute(
                "DELETE FROM simplefin_pending_txn WHERE EXISTS \
                 (SELECT 1 FROM txn WHERE txn.simplefin_id = simplefin_pending_txn.simplefin_id \
                  OR txn.simplefin_counterpart_id = simplefin_pending_txn.simplefin_id)",
                (),
            )
            .await
            .map_err(|e| e.to_string())? as i64;
        summary.pending_changed = cleared > 0;
    }

    Ok((summary, cache))
}

/// Mirrors INVESTMENT_TYPES in src/lib/accountTypes.ts.
fn is_investment_account_type(t: &str) -> bool {
    matches!(
        t,
        "brokerage" | "401k" | "roth_401k" | "mixed_401k" | "traditional_ira" | "roth_ira"
            | "hsa" | "crypto"
    )
}

/// Collapse SimpleFIN-imported transfer pairs into the app's canonical
/// single-row transfer model.
///
/// SimpleFIN's protocol has no cross-account linkage, so a real transfer
/// between two linked accounts always arrives as two independent rows: an
/// expense on the source account and an income on the destination. This pass
/// matches such pairs by amount (± float tolerance) and a date window,
/// deliberately ignoring description (the two sides almost never share
/// wording). On a match the source (expense) row survives — the canonical
/// model is `account_id` = source, `transfer_account_id` = destination — and
/// the destination row is deleted, its SimpleFIN id preserved in
/// `simplefin_counterpart_id` so the import's NOT EXISTS guard blocks its
/// re-import from the overlap window.
///
/// A transfer into an investment account is additionally flagged as a
/// contribution — but only when the source is NOT itself an investment
/// account: moving money between two investment accounts (e.g. a rollover)
/// is not new principal, and counting it would inflate the contribution rate
/// feeding the FIRE forecast.
///
/// Scans all SimpleFIN rows (not just this sync's) so a counterpart that
/// arrives days later — or history predating this feature — still collapses.
/// Rows a user has wired into the balance chain (generated snapshots) or that
/// belong to a paycheck are left alone. This is a best-effort heuristic: two
/// unrelated same-amount transactions across accounts within the window will
/// also match.
pub(crate) async fn collapse_transfer_pairs(conn: &Connection) -> Result<i64, String> {
    struct Pair {
        expense_id: i32,
        income_id: i32,
        income_sfin_id: String,
        is_contribution: bool,
    }

    // Candidates ordered by date proximity so the greedy pass below pairs each
    // row with its closest match first.
    let mut rows = conn
        .query(
            "SELECT e.id, i.id, i.simplefin_id, ea.type, ia.type \
             FROM txn e \
             JOIN txn i ON i.type = 'income' \
               AND i.import_source = 'simplefin' \
               AND i.simplefin_id IS NOT NULL \
               AND i.account_id <> e.account_id \
               AND ABS(i.amount - e.amount) < 0.005 \
               AND ABS(julianday(i.date) - julianday(e.date)) <= ?1 \
             JOIN account ea ON ea.id = e.account_id \
             JOIN account ia ON ia.id = i.account_id \
             WHERE e.type = 'expense' \
               AND e.import_source = 'simplefin' \
               AND e.simplefin_id IS NOT NULL \
               AND e.suppressed_as IS NULL AND i.suppressed_as IS NULL \
               AND e.paycheck_id IS NULL AND i.paycheck_id IS NULL \
               AND e.generated_balance_id IS NULL AND e.generated_balance_to_id IS NULL \
               AND i.generated_balance_id IS NULL AND i.generated_balance_to_id IS NULL \
             ORDER BY ABS(julianday(i.date) - julianday(e.date)) ASC, e.id ASC, i.id ASC",
            params![TRANSFER_DATE_TOLERANCE_DAYS],
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut candidates: Vec<Pair> = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        let src_type: String = r.get(3).map_err(|e| e.to_string())?;
        let dst_type: String = r.get(4).map_err(|e| e.to_string())?;
        candidates.push(Pair {
            expense_id: r.get(0).map_err(|e| e.to_string())?,
            income_id: r.get(1).map_err(|e| e.to_string())?,
            income_sfin_id: r.get(2).map_err(|e| e.to_string())?,
            is_contribution: is_investment_account_type(&dst_type)
                && !is_investment_account_type(&src_type),
        });
    }

    // Greedy one-to-one matching: each row participates in at most one pair.
    let mut used_expenses = std::collections::HashSet::new();
    let mut used_incomes = std::collections::HashSet::new();
    let updated_at = now_iso();
    let mut collapsed = 0i64;

    for p in candidates {
        if used_expenses.contains(&p.expense_id) || used_incomes.contains(&p.income_id) {
            continue;
        }
        used_expenses.insert(p.expense_id);
        used_incomes.insert(p.income_id);

        // The surviving row's category no longer means anything (transfers are
        // cash-flow neutral); reset it like the CSV importer does.
        conn.execute(
            "UPDATE txn SET type = 'transfer', \
             transfer_account_id = (SELECT account_id FROM txn WHERE id = ?1), \
             category = 'uncategorized', is_contribution = ?2, \
             simplefin_counterpart_id = ?3, updated_at = ?4 \
             WHERE id = ?5",
            params![p.income_id, p.is_contribution, p.income_sfin_id, updated_at.clone(), p.expense_id],
        )
        .await
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM txn WHERE id = ?1", params![p.income_id])
            .await
            .map_err(|e| e.to_string())?;
        collapsed += 1;
    }

    Ok(collapsed)
}

/// Absorb a SimpleFIN-imported income/expense row into an existing transfer
/// the user converted by hand.
///
/// When only one side of a transfer has posted, its row sits as ordinary
/// income/expense until the counterpart arrives and `collapse_transfer_pairs`
/// merges them. A user who converts that row to a transfer early (rather than
/// waiting out the lag) leaves nothing for the collapse pass to pair with —
/// the counterpart would import as a fresh row and double-count the flow.
///
/// Such a hand-converted transfer is recognizable: it still carries its own
/// `simplefin_id` but has no `simplefin_counterpart_id` (only the collapse
/// pass writes that column). This pass matches an incoming SimpleFIN row
/// against those transfers by amount (± float tolerance), date window, and
/// account orientation — an expense must sit on the transfer's source
/// account, an income on its destination. On a match the incoming row is
/// deleted and its id stored in `simplefin_counterpart_id`, exactly as if the
/// collapse pass had paired the two, so the import's NOT EXISTS guard blocks
/// re-import from the overlap window.
pub(crate) async fn absorb_into_user_transfers(conn: &Connection) -> Result<i64, String> {
    struct Match {
        incoming_id: i32,
        incoming_sfin_id: String,
        transfer_id: i32,
    }

    // Candidates ordered by date proximity so the greedy pass below pairs each
    // row with its closest match first (same technique as collapse_transfer_pairs).
    let mut rows = conn
        .query(
            "SELECT s.id, s.simplefin_id, tr.id \
             FROM txn s \
             JOIN txn tr ON tr.type = 'transfer' \
               AND tr.import_source = 'simplefin' \
               AND tr.simplefin_id IS NOT NULL \
               AND tr.simplefin_counterpart_id IS NULL \
               AND tr.simplefin_id <> s.simplefin_id \
               AND tr.suppressed_as IS NULL \
               AND ABS(tr.amount - s.amount) < 0.005 \
               AND ABS(julianday(tr.date) - julianday(s.date)) <= ?1 \
               AND ((s.type = 'expense' AND s.account_id = tr.account_id) \
                 OR (s.type = 'income' AND s.account_id = tr.transfer_account_id)) \
             WHERE s.type IN ('income', 'expense') \
               AND s.import_source = 'simplefin' \
               AND s.simplefin_id IS NOT NULL \
               AND s.suppressed_as IS NULL \
               AND s.paycheck_id IS NULL \
               AND s.generated_balance_id IS NULL AND s.generated_balance_to_id IS NULL \
             ORDER BY ABS(julianday(tr.date) - julianday(s.date)) ASC, s.id ASC, tr.id ASC",
            params![TRANSFER_DATE_TOLERANCE_DAYS],
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut candidates: Vec<Match> = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        candidates.push(Match {
            incoming_id: r.get(0).map_err(|e| e.to_string())?,
            incoming_sfin_id: r.get(1).map_err(|e| e.to_string())?,
            transfer_id: r.get(2).map_err(|e| e.to_string())?,
        });
    }

    // Greedy one-to-one matching: each row participates in at most one pair.
    let mut used_incoming = std::collections::HashSet::new();
    let mut used_transfers = std::collections::HashSet::new();
    let updated_at = now_iso();
    let mut absorbed = 0i64;

    for m in candidates {
        if used_incoming.contains(&m.incoming_id) || used_transfers.contains(&m.transfer_id) {
            continue;
        }
        used_incoming.insert(m.incoming_id);
        used_transfers.insert(m.transfer_id);

        conn.execute(
            "UPDATE txn SET simplefin_counterpart_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![m.incoming_sfin_id, updated_at.clone(), m.transfer_id],
        )
        .await
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM txn WHERE id = ?1", params![m.incoming_id])
            .await
            .map_err(|e| e.to_string())?;
        absorbed += 1;
    }

    Ok(absorbed)
}

/// Find candidate duplicate pairs for the on-demand review: same account, same
/// type, matching amount (± float tolerance), dates within a small window, one
/// side SimpleFIN-imported and the other not.
///
/// Description is deliberately NOT matched — a hand-typed "Starbucks" vs.
/// SimpleFIN's "STARBUCKS #123 SEATTLE WA" is the same purchase. The query's
/// job is recall; both descriptions are shown side by side in the review UI so
/// the human eye handles precision (false positives get unchecked, nothing is
/// deleted without the user submitting).
///
/// Transfers are excluded entirely: the app stores a transfer as one canonical
/// row while SimpleFIN reports two independent ordinary rows, so the shapes
/// don't line up for pair-matching (and `collapse_transfer_pairs` already
/// handles the SimpleFIN side).
pub(crate) async fn duplicate_candidates(
    conn: &Connection,
) -> Result<Vec<SimpleFinDuplicateCandidate>, String> {
    // Candidates ordered by date proximity so the greedy pass below pairs each
    // row with its closest match first (same technique as collapse_transfer_pairs).
    let mut rows = conn
        .query(
            "SELECT s.id, s.date, s.description, m.id, m.date, m.description, \
             m.import_source, m.paycheck_id, s.amount, s.type, a.id, a.name, a.type \
             FROM txn s \
             JOIN txn m ON m.account_id = s.account_id \
               AND m.simplefin_id IS NULL \
               AND m.type = s.type \
               AND ABS(m.amount - s.amount) < 0.005 \
               AND ABS(julianday(m.date) - julianday(s.date)) <= ?1 \
             JOIN account a ON a.id = s.account_id \
             WHERE s.simplefin_id IS NOT NULL \
               AND s.type IN ('income', 'expense') \
               AND s.suppressed_as IS NULL AND m.suppressed_as IS NULL \
             ORDER BY ABS(julianday(m.date) - julianday(s.date)) ASC, s.id ASC, m.id ASC",
            params![DUPLICATE_DATE_TOLERANCE_DAYS],
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut candidates: Vec<SimpleFinDuplicateCandidate> = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        let paycheck_id: Option<i32> = r.get(7).map_err(|e| e.to_string())?;
        let account_type: String = r.get(12).map_err(|e| e.to_string())?;
        let bucket = match paycheck_id {
            Some(_) if is_investment_account_type(&account_type) => "contribution",
            Some(_) => "net_deposit",
            None => "ordinary",
        };
        candidates.push(SimpleFinDuplicateCandidate {
            simplefin_txn_id: r.get(0).map_err(|e| e.to_string())?,
            simplefin_date: r.get(1).map_err(|e| e.to_string())?,
            simplefin_description: r.get(2).map_err(|e| e.to_string())?,
            other_txn_id: r.get(3).map_err(|e| e.to_string())?,
            other_date: r.get(4).map_err(|e| e.to_string())?,
            other_description: r.get(5).map_err(|e| e.to_string())?,
            other_import_source: r.get(6).map_err(|e| e.to_string())?,
            amount: r.get(8).map_err(|e| e.to_string())?,
            txn_type: r.get(9).map_err(|e| e.to_string())?,
            account_id: r.get(10).map_err(|e| e.to_string())?,
            account_name: r.get(11).map_err(|e| e.to_string())?,
            bucket: bucket.to_string(),
        });
    }

    // Greedy one-to-one matching: each transaction appears in at most one pair.
    let mut used_sfin = std::collections::HashSet::new();
    let mut used_other = std::collections::HashSet::new();
    candidates.retain(|c| {
        if used_sfin.contains(&c.simplefin_txn_id) || used_other.contains(&c.other_txn_id) {
            return false;
        }
        used_sfin.insert(c.simplefin_txn_id);
        used_other.insert(c.other_txn_id);
        true
    });

    Ok(candidates)
}

// ---- sync orchestration ----

/// The single funnel all SimpleFIN sync triggers call.
///
/// Every phase that touches the database runs under the global `DbGate`, so an
/// import can never overlap a Turso `db.sync()` rewriting the replica file
/// (which surfaced as "SQLite failure: `file is not a database`"). The network
/// fetch deliberately happens OUTSIDE the gate — a slow bank response must not
/// block cloud sync.
pub async fn run_sync(app: &AppHandle) -> Result<SimpleFinSyncSummary, String> {
    run_sync_inner(app, None).await
}

/// `range`: None for a regular sync (rolling window + pending cache rebuild);
/// Some((start_ts, end_ts_exclusive)) for a user-requested backfill of a
/// specific window (posted transactions only, pending cache left alone).
async fn run_sync_inner(
    app: &AppHandle,
    range: Option<(i64, i64)>,
) -> Result<SimpleFinSyncSummary, String> {
    let shared = app.state::<SimpleFinShared>();
    let _guard = shared.lock.lock().await; // serialize scheduler vs. manual click

    let access_url = access_url_get()?
        .ok_or_else(|| "SimpleFIN is not connected on this device.".to_string())?;
    let db = app.state::<Db>();
    let conn = db.conn().await?;
    let gate = app.state::<crate::sync::DbGate>();

    let _ = app.emit(
        "simplefin-syncing",
        SimpleFinSyncingEvent { syncing: true, error: None, transactions_added: 0, snapshots_added: 0 },
    );

    match range {
        Some((s, e)) => crate::sync::sync_log(
            app,
            &format!("simplefin: custom-range sync start ({} .. {})", ts_to_date(s), ts_to_date(e)),
        ),
        None => crate::sync::sync_log(app, "simplefin: sync start"),
    }

    // Everything fallible runs inside this block so the finish event below is
    // emitted on every path — a leaked `?` here would strand the frontend's
    // "syncing…" notification.
    let result: Result<SimpleFinSyncSummary, String> = async {
        let (state, oldest_pending) = {
            let _db_guard = gate.lock.lock().await;
            let state = read_state(&conn).await?;
            // Oldest cached pending date — read BEFORE the import wipes the
            // cache; it can anchor the window further back than the overlap.
            let oldest_pending: Option<String> = {
                let mut rows = conn
                    .query("SELECT MIN(date) FROM simplefin_pending_txn", ())
                    .await
                    .map_err(|e| e.to_string())?;
                match rows.next().await.map_err(|e| e.to_string())? {
                    Some(r) => r.get(0).map_err(|e| e.to_string())?,
                    None => None,
                }
            };
            set_state(&conn, "last_attempt_at", Some(&now_iso())).await?;
            (state, oldest_pending)
        };

        // Transaction window: always overlap back past the last SUCCESS (so a
        // connection that was broken for days or weeks backfills the whole
        // gap) and past the oldest cached pending transaction (so a slow-to-
        // post charge can't slide out of the window and vanish) — unless the
        // user requested an explicit range.
        let (start_ts, end_ts) = match range {
            Some((s, e)) => (s, Some(e)),
            None => (
                window_start(
                    Utc::now(),
                    state.last_success_at.as_deref().and_then(parse_iso),
                    oldest_pending.as_deref(),
                ),
                None,
            ),
        };

        // Network fetch — no DB access, no gate.
        let (set, raw) = fetch_accounts(&access_url, Some(start_ts), end_ts, range.is_none()).await?;
        crate::sync::sync_log(app, &format!("simplefin: fetch ok ({} accounts)", set.accounts.len()));
        dump_raw_response(app, &raw);

        let _db_guard = gate.lock.lock().await;
        // Lock/busy failures (e.g. a WAL checkpoint colliding with a straggling
        // reader) are transient; the import is idempotent by construction
        // (INSERT OR IGNORE + same-day snapshot dedup), so one retry is safe.
        let rebuild_pending = range.is_none();
        let (summary, cache) = match import_account_set(&conn, &set, rebuild_pending).await {
            Ok(v) => v,
            Err(e) if is_transient_lock_error(&e) => {
                crate::sync::sync_log(
                    app,
                    &format!("simplefin: import hit transient lock, retrying once: {e}"),
                );
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                import_account_set(&conn, &set, rebuild_pending).await?
            }
            Err(e) => return Err(e),
        };
        let accounts_json = serde_json::to_string(&cache).map_err(|e| e.to_string())?;
        let bridge_json = serde_json::to_string(&set.errors).map_err(|e| e.to_string())?;
        set_state(&conn, "accounts_json", Some(&accounts_json)).await?;
        set_state(&conn, "bridge_errors", Some(&bridge_json)).await?;
        set_state(&conn, "last_success_at", Some(&now_iso())).await?;
        set_state(&conn, "last_error", None).await?;
        Ok(summary)
    }
    .await;

    match &result {
        Ok(s) => crate::sync::sync_log(
            app,
            &format!(
                "simplefin: sync ok ({} txns, {} snapshots)",
                s.transactions_added, s.snapshots_added
            ),
        ),
        Err(e) => crate::sync::sync_log(app, &format!("simplefin: sync failed: {e}")),
    }

    // The corruption signature means the whole app is about to start failing —
    // verify + reopen the database in place before anything else reads it.
    if let Err(e) = &result {
        if Db::is_corruption_error(e) {
            let _db_guard = gate.lock.lock().await;
            crate::sync::check_health(app, "simplefin: post-failure").await;
        }
    }

    {
        let _db_guard = gate.lock.lock().await;
        if let Err(e) = &result {
            // Re-fetch the (possibly refreshed) shared connection so the error
            // write works even after a recovery above.
            if let Ok(conn) = db.conn().await {
                let _ = set_state(&conn, "last_error", Some(e)).await;
            }
        }
        // Tell the frontend. Data pages remount only when something changed.
        if let Ok(conn) = db.conn().await {
            if let Ok(status) = build_status(&conn).await {
                let _ = app.emit("simplefin-status", status);
            }
        }
    }
    let _ = app.emit(
        "simplefin-syncing",
        SimpleFinSyncingEvent {
            syncing: false,
            error: result.as_ref().err().cloned(),
            transactions_added: result.as_ref().map(|s| s.transactions_added).unwrap_or(0),
            snapshots_added: result.as_ref().map(|s| s.snapshots_added).unwrap_or(0),
        },
    );
    if let Ok(s) = &result {
        if s.transactions_added > 0 || s.snapshots_added > 0 || s.pending_changed {
            let _ = app.emit("data-refreshed", ());
        }
    }

    result
}

/// Scheduler entry point: sync only when due. Errors are recorded in the state
/// row by `run_sync`; the scheduler itself never fails.
pub async fn maybe_sync(app: &AppHandle) {
    // Auto-sync only while the app is focused: a backgrounded app doesn't need
    // fresh bank data, and skipping keeps us far under SimpleFIN's request
    // budget. The focus handler in `lib.rs` re-enters here the moment focus
    // returns, so a stale app catches up immediately on focus rather than
    // waiting for the next scheduler tick.
    let focused = app.webview_windows().values().any(|w| w.is_focused().unwrap_or(false));
    if !focused {
        return;
    }
    // A focus event can in principle fire before `setup` finishes managing
    // state; `run_sync` needs all three, so bail instead of panicking.
    if app.try_state::<Db>().is_none()
        || app.try_state::<SimpleFinShared>().is_none()
        || app.try_state::<crate::sync::DbGate>().is_none()
    {
        return;
    }
    let Ok(Some(_)) = access_url_get() else { return };
    let Ok(conn) = app.state::<Db>().conn().await else { return };
    let Ok(state) = read_state(&conn).await else { return };
    let due = sync_due(
        Utc::now(),
        state.last_success_at.as_deref().and_then(parse_iso),
        state.last_attempt_at.as_deref().and_then(parse_iso),
    );
    if due {
        let _ = run_sync(app).await;
    }
}

async fn build_status(conn: &Connection) -> Result<SimpleFinStatus, String> {
    let connected = access_url_get().unwrap_or(None).is_some();
    let state = read_state(conn).await?;

    let mut accounts: Vec<SimpleFinRemoteAccount> = state
        .accounts_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    // Fill link info fresh from the account table.
    let mut rows = conn
        .query(
            "SELECT simplefin_id, id FROM account WHERE simplefin_id IS NOT NULL",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut links: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        links.insert(
            r.get::<String>(0).map_err(|e| e.to_string())?,
            r.get::<i32>(1).map_err(|e| e.to_string())?,
        );
    }
    for a in &mut accounts {
        a.linked_account_id = links.get(&a.id).copied();
    }

    Ok(SimpleFinStatus {
        connected,
        claimed_at: state.claimed_at,
        last_attempt_at: state.last_attempt_at,
        last_success_at: state.last_success_at,
        last_error: state.last_error,
        bridge_errors: state
            .bridge_errors
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default(),
        accounts,
    })
}

// ---- commands ----

#[tauri::command]
pub async fn simplefin_get_status(app: AppHandle) -> Result<SimpleFinStatus, String> {
    let conn = app.state::<Db>().conn().await?;
    build_status(&conn).await
}

/// Claim the setup token, store the access URL in the keychain, and run an
/// immediate first sync (populates the remote-account cache for the linking
/// UI). A failed first fetch does not lose the connection — the access URL is
/// already stored and the error is surfaced in the returned status.
#[tauri::command]
pub async fn simplefin_connect(
    app: AppHandle,
    setup_token: String,
) -> Result<SimpleFinStatus, String> {
    let access_url = claim_access_url(&setup_token).await?;
    access_url_set(&access_url)?;
    let conn = app.state::<Db>().conn().await?;
    set_state(&conn, "claimed_at", Some(&now_iso())).await?;
    let _ = run_sync(&app).await;
    build_status(&conn).await
}

/// Link (or unlink, with `account_id = None`) a SimpleFIN account to a local
/// account. Transactions and balances import on the next sync.
#[tauri::command]
pub async fn simplefin_link_account(
    app: AppHandle,
    simplefin_id: String,
    account_id: Option<i32>,
) -> Result<(), String> {
    let conn = app.state::<Db>().conn().await?;
    conn.execute(
        "UPDATE account SET simplefin_id = NULL WHERE simplefin_id = ?1",
        params![simplefin_id.clone()],
    )
    .await
    .map_err(|e| e.to_string())?;
    if let Some(id) = account_id {
        conn.execute(
            "UPDATE account SET simplefin_id = ?1 WHERE id = ?2",
            params![simplefin_id, id],
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn simplefin_sync_now(app: AppHandle) -> Result<SimpleFinSyncSummary, String> {
    run_sync(&app).await
}

/// User-requested backfill of an explicit date range (inclusive on both ends,
/// "YYYY-MM-DD"). Posted transactions only; the pending cache is left alone.
/// Dedup makes any overlap with already-imported history harmless.
#[tauri::command]
pub async fn simplefin_sync_range(
    app: AppHandle,
    start_date: String,
    end_date: String,
) -> Result<SimpleFinSyncSummary, String> {
    let range = parse_custom_range(&start_date, &end_date, Local::now().date_naive())?;
    run_sync_inner(&app, Some(range)).await
}

/// List candidate SimpleFIN-vs-manual/CSV duplicate pairs for the review UI.
/// Read-only — resolution happens through the transaction delete commands.
#[tauri::command]
pub async fn simplefin_duplicate_candidates(
    app: AppHandle,
) -> Result<Vec<SimpleFinDuplicateCandidate>, String> {
    let conn = app.state::<Db>().conn().await?;
    duplicate_candidates(&conn).await
}

/// List the cached pending transactions (newest first). Awareness only —
/// these rows are outside the ledger and every sum/aggregate.
#[tauri::command]
pub async fn simplefin_list_pending(
    app: AppHandle,
) -> Result<Vec<SimpleFinPendingTransaction>, String> {
    let conn = app.state::<Db>().conn().await?;
    let mut rows = conn
        .query(
            "SELECT id, account_id, amount, description, date, type, simplefin_id, \
             raw_description \
             FROM simplefin_pending_txn ORDER BY date DESC, id DESC",
            (),
        )
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().await.map_err(|e| e.to_string())? {
        out.push(SimpleFinPendingTransaction {
            id: r.get(0).map_err(|e| e.to_string())?,
            account_id: r.get(1).map_err(|e| e.to_string())?,
            amount: r.get(2).map_err(|e| e.to_string())?,
            description: r.get(3).map_err(|e| e.to_string())?,
            date: r.get(4).map_err(|e| e.to_string())?,
            txn_type: r.get(5).map_err(|e| e.to_string())?,
            simplefin_id: r.get(6).map_err(|e| e.to_string())?,
            raw_description: r.get(7).map_err(|e| e.to_string())?,
        });
    }
    Ok(out)
}

/// Remove the access URL from the keychain and reset sync state. Imported data
/// and account links are kept — reconnecting resumes where things left off,
/// and the provenance flags stay useful for debugging.
#[tauri::command]
pub async fn simplefin_disconnect(app: AppHandle) -> Result<(), String> {
    access_url_delete()?;
    let conn = app.state::<Db>().conn().await?;
    for col in ["claimed_at", "last_attempt_at", "last_success_at", "last_error", "bridge_errors", "accounts_json"] {
        set_state(&conn, col, None).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use libsql::Builder;

    #[test]
    fn decodes_base64_setup_token() {
        let url = "https://bridge.simplefin.org/simplefin/claim/DEMO";
        let token = base64::engine::general_purpose::STANDARD.encode(url);
        assert_eq!(decode_setup_token(&token).unwrap(), url);
        // Whitespace/newlines from copy-paste are tolerated.
        let wrapped = format!("  {}\n", token);
        assert_eq!(decode_setup_token(&wrapped).unwrap(), url);
        // A raw claim URL passes through.
        assert_eq!(decode_setup_token(url).unwrap(), url);
        // Garbage is rejected.
        assert!(decode_setup_token("not-a-token!!!").is_err());
    }

    #[test]
    fn description_skips_processor_wrappers_and_keeps_raw() {
        let mk = |payee: Option<&str>, desc: Option<&str>, memo: Option<&str>| SfinTxn {
            id: "TRN-1".into(),
            posted: 1,
            transacted_at: None,
            amount: NumOrStr::S("-1.00".into()),
            description: desc.map(str::to_string),
            payee: payee.map(str::to_string),
            memo: memo.map(str::to_string),
            pending: None,
            category: None,
            extra: None,
        };

        // The real-world shape: a payee that is only the processor name and a
        // description wrapped in the processor prefix. The wrapper is stripped
        // and the bare payee never wins; the raw keeps both, unedited.
        let (display, raw) = select_description(&mk(
            Some("Link.com"),
            Some("LINK.COM* SIMPLEFIN BR SOUTH SAN FRA USA"),
            None,
        ));
        assert_eq!(display, "SIMPLEFIN BR SOUTH SAN FRA USA");
        assert_eq!(
            raw.as_deref(),
            Some("Link.com · LINK.COM* SIMPLEFIN BR SOUTH SAN FRA USA")
        );

        // Ordinary transactions are untouched, and duplicate fields collapse
        // in the raw.
        let (display, raw) = select_description(&mk(Some("Aldi"), Some("ALDI"), None));
        assert_eq!(display, "Aldi");
        assert_eq!(raw.as_deref(), Some("Aldi"));

        // All fields being just the processor name still yields a display.
        let (display, raw) = select_description(&mk(Some("Link.com"), None, None));
        assert_eq!(display, "SimpleFIN transaction");
        assert_eq!(raw.as_deref(), Some("Link.com"));

        // Nothing at all.
        let (display, raw) = select_description(&mk(None, None, None));
        assert_eq!(display, "SimpleFIN transaction");
        assert_eq!(raw, None);
    }

    #[test]
    fn sync_due_backs_off_after_failures() {
        let now = Utc::now();
        let h = chrono::Duration::hours;
        // Never synced → due.
        assert!(sync_due(now, None, None));
        // Succeeded 2h ago → not due (interval is 3h).
        assert!(!sync_due(now, Some(now - h(2)), Some(now - h(2))));
        // Succeeded 4h ago (attempt recorded just before the success) → due;
        // a successful attempt must not trigger the failure backoff.
        assert!(sync_due(now, Some(now - h(4)), Some(now - h(4))));
        // Succeeded 25h ago but a (failed) attempt 2h ago → back off.
        assert!(!sync_due(now, Some(now - h(25)), Some(now - h(2))));
        // ...and retry once the failed attempt is 6h old.
        assert!(sync_due(now, Some(now - h(30)), Some(now - h(7))));
        // Never succeeded, failed attempt 2h ago → back off.
        assert!(!sync_due(now, None, Some(now - h(2))));
    }

    #[test]
    fn window_start_anchors_to_oldest_pending() {
        let now = Utc::now();
        let d = chrono::Duration::days;
        let base = window_start(now, Some(now), None);
        assert_eq!(base, now.timestamp() - OVERLAP_DAYS * 86_400);

        // First sync: 90-day lookback.
        assert_eq!(window_start(now, None, None), now.timestamp() - FIRST_SYNC_LOOKBACK_DAYS * 86_400);

        // A pending row older than the overlap pulls the window back to a day
        // before its date — a slow-to-post charge must stay inside the window.
        let old_pending = ts_to_date((now - d(6)).timestamp());
        let anchored = window_start(now, Some(now), Some(&old_pending));
        assert!(anchored < base);
        assert!(anchored <= date_to_ts(&old_pending).unwrap() - PENDING_ANCHOR_BUFFER_DAYS * 86_400);

        // A pending row inside the overlap changes nothing.
        let fresh_pending = ts_to_date(now.timestamp());
        assert_eq!(window_start(now, Some(now), Some(&fresh_pending)), base);

        // Garbage dates are ignored rather than breaking the sync.
        assert_eq!(window_start(now, Some(now), Some("not-a-date")), base);
    }

    #[test]
    fn custom_range_validates_and_is_end_inclusive() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 7).unwrap();

        let (start, end) = parse_custom_range("2026-07-01", "2026-07-05", today).unwrap();
        assert_eq!(start, date_to_ts("2026-07-01").unwrap());
        // Exclusive end timestamp = start of the day AFTER the chosen end.
        assert_eq!(end, date_to_ts("2026-07-06").unwrap());

        // Single-day range is allowed.
        assert!(parse_custom_range("2026-07-05", "2026-07-05", today).is_ok());
        // Reversed range.
        assert!(parse_custom_range("2026-07-05", "2026-07-01", today).is_err());
        // End in the future.
        assert!(parse_custom_range("2026-07-01", "2026-07-08", today).is_err());
        // Start older than the max lookback.
        assert!(parse_custom_range("2025-01-01", "2026-07-05", today).is_err());
        // Garbage.
        assert!(parse_custom_range("07/01/2026", "2026-07-05", today).is_err());
    }

    #[test]
    fn extracts_vendor_category() {
        let mk = |category: Option<&str>, extra: Option<serde_json::Value>| SfinTxn {
            id: "TRN-1".into(),
            posted: 1,
            transacted_at: None,
            amount: NumOrStr::S("-1.00".into()),
            description: None,
            payee: None,
            memo: None,
            pending: None,
            category: category.map(str::to_string),
            extra,
        };
        assert_eq!(vendor_category(&mk(Some("Groceries"), None)).as_deref(), Some("Groceries"));
        assert_eq!(
            vendor_category(&mk(None, Some(serde_json::json!({"category": "Dining"}))))
                .as_deref(),
            Some("Dining")
        );
        assert_eq!(
            vendor_category(&mk(
                None,
                Some(serde_json::json!({"categories": ["Food", "Coffee"]}))
            ))
            .as_deref(),
            Some("Food / Coffee")
        );
        assert_eq!(vendor_category(&mk(None, None)), None);
    }

    async fn setup_db() -> Connection {
        let db = Builder::new_local(":memory:").build().await.unwrap();
        let conn = db.connect().unwrap();
        crate::migrations::run(&conn).await.unwrap();
        conn
    }

    async fn insert_account(conn: &Connection, name: &str, ty: &str, sfin: &str) -> i32 {
        conn.execute(
            "INSERT INTO account (name, type, is_active, include_in_fire_calculations, \
             created_at, simplefin_id) VALUES (?1, ?2, 1, 0, '2024-01-01', ?3)",
            params![name, ty, sfin],
        )
        .await
        .unwrap();
        conn.last_insert_rowid() as i32
    }

    fn demo_set(balance: &str, balance_date: i64, txns: Vec<SfinTxn>) -> SfinAccountSet {
        SfinAccountSet {
            errors: vec![],
            accounts: vec![SfinAccount {
                id: "ACT-1".into(),
                name: Some("Demo Checking".into()),
                org: None,
                currency: Some(serde_json::json!("USD")),
                balance: NumOrStr::S(balance.into()),
                balance_date,
                transactions: txns,
            }],
        }
    }

    fn demo_txn(id: &str, amount: &str, posted: i64, desc: &str) -> SfinTxn {
        SfinTxn {
            id: id.into(),
            posted,
            transacted_at: None,
            amount: NumOrStr::S(amount.into()),
            description: Some(desc.into()),
            payee: None,
            memo: None,
            pending: None,
            category: None,
            extra: None,
        }
    }

    /// Two linked accounts, each with its own transactions.
    fn demo_set2(txns_a: Vec<SfinTxn>, txns_b: Vec<SfinTxn>) -> SfinAccountSet {
        SfinAccountSet {
            errors: vec![],
            accounts: vec![
                SfinAccount {
                    id: "ACT-1".into(),
                    name: Some("Checking".into()),
                    org: None,
                    currency: Some(serde_json::json!("USD")),
                    balance: NumOrStr::S("1000.00".into()),
                    balance_date: 1_700_000_000,
                    transactions: txns_a,
                },
                SfinAccount {
                    id: "ACT-2".into(),
                    name: Some("Other".into()),
                    org: None,
                    currency: Some(serde_json::json!("USD")),
                    balance: NumOrStr::S("5000.00".into()),
                    balance_date: 1_700_000_000,
                    transactions: txns_b,
                },
            ],
        }
    }

    // Reads (account_id, transfer_account_id, amount, type, is_contribution,
    // simplefin_id, simplefin_counterpart_id) for all txns, ordered by id.
    async fn read_all_txns(
        conn: &Connection,
    ) -> Vec<(i32, Option<i32>, f64, String, bool, Option<String>, Option<String>)> {
        let mut rows = conn
            .query(
                "SELECT account_id, transfer_account_id, amount, type, is_contribution, \
                 simplefin_id, simplefin_counterpart_id FROM txn ORDER BY id",
                (),
            )
            .await
            .unwrap();
        let mut out = Vec::new();
        while let Some(r) = rows.next().await.unwrap() {
            out.push((
                r.get::<i32>(0).unwrap(),
                r.get::<Option<i32>>(1).unwrap(),
                r.get::<f64>(2).unwrap(),
                r.get::<String>(3).unwrap(),
                r.get::<i64>(4).unwrap() != 0,
                r.get::<Option<String>>(5).unwrap(),
                r.get::<Option<String>>(6).unwrap(),
            ));
        }
        out
    }

    const DAY: i64 = 86_400;

    /// Insert a non-SimpleFIN txn row directly (manual/CSV/paycheck shapes).
    async fn insert_local_txn(
        conn: &Connection,
        account_id: i32,
        ty: &str,
        amount: f64,
        date: &str,
        description: &str,
        import_source: &str,
        paycheck_id: Option<i32>,
    ) -> i32 {
        conn.execute(
            "INSERT INTO txn (account_id, transfer_account_id, amount, description, date, \
             type, category, is_contribution, is_withdrawal, import_source, paycheck_id, \
             created_at, updated_at) \
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, 'uncategorized', 0, 0, ?6, ?7, \
             '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            params![account_id, amount, description, date, ty, import_source, paycheck_id],
        )
        .await
        .unwrap();
        conn.last_insert_rowid() as i32
    }

    #[tokio::test]
    async fn duplicate_candidates_matches_across_description_drift() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;

        let ts = 1_700_000_000;
        // Manual entry dated a day before the bank's posted date, hand-typed
        // description — must still match (amount + type + date window only).
        let manual = insert_local_txn(
            &conn, acc, "expense", 42.50, &ts_to_date(ts - DAY), "Starbucks", "manual", None,
        )
        .await;
        let set = demo_set(
            "1500.00",
            ts,
            vec![demo_txn("TRN-1", "-42.50", ts, "STARBUCKS #123 SEATTLE WA")],
        );
        import_account_set(&conn, &set, true).await.unwrap();

        let cands = duplicate_candidates(&conn).await.unwrap();
        assert_eq!(cands.len(), 1);
        let c = &cands[0];
        assert_eq!(c.other_txn_id, manual);
        assert_eq!(c.bucket, "ordinary");
        assert_eq!(c.amount, 42.50);
        assert_eq!(c.txn_type, "expense");
        assert_eq!(c.simplefin_description, "STARBUCKS #123 SEATTLE WA");
        assert_eq!(c.other_description, "Starbucks");
        assert_eq!(c.account_name, "Checking");
    }

    #[tokio::test]
    async fn duplicate_candidates_respects_type_date_and_amount_bounds() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;

        let ts = 1_700_000_000;
        // Same amount but opposite type: never a duplicate.
        insert_local_txn(&conn, acc, "income", 42.50, &ts_to_date(ts), "refund", "manual", None)
            .await;
        // Same amount/type but 5 days out: outside the window.
        insert_local_txn(
            &conn, acc, "expense", 42.50, &ts_to_date(ts + 5 * DAY), "later", "manual", None,
        )
        .await;
        // Same type/date but different amount.
        insert_local_txn(&conn, acc, "expense", 43.00, &ts_to_date(ts), "close", "manual", None)
            .await;
        // A manual transfer row is excluded even with matching amount/date.
        insert_local_txn(&conn, acc, "transfer", 42.50, &ts_to_date(ts), "move", "manual", None)
            .await;

        let set = demo_set("1500.00", ts, vec![demo_txn("TRN-1", "-42.50", ts, "CHARGE")]);
        import_account_set(&conn, &set, true).await.unwrap();

        assert!(duplicate_candidates(&conn).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn duplicate_candidates_classifies_paycheck_buckets() {
        let conn = setup_db().await;
        let checking = insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let k401 = insert_account(&conn, "401k", "401k", "ACT-2").await;

        let ts = 1_700_000_000;
        conn.execute(
            "INSERT INTO paycheck (pay_date, employer, pay_period, gross_amount, net_amount, \
             created_at, updated_at) VALUES (?1, 'Acme', 'biweekly', 3500.0, 2500.0, \
             '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            params![ts_to_date(ts)],
        )
        .await
        .unwrap();
        // Paycheck net deposit on checking, and a contribution row on the 401k.
        insert_local_txn(
            &conn, checking, "income", 2500.0, &ts_to_date(ts), "Paycheck – Acme", "paycheck",
            Some(1),
        )
        .await;
        insert_local_txn(
            &conn, k401, "income", 500.0, &ts_to_date(ts), "401k deduction", "paycheck", Some(1),
        )
        .await;

        let set = demo_set2(
            vec![demo_txn("TRN-DEP", "2500.00", ts, "ACME PAYROLL")],
            vec![demo_txn("TRN-CONTRIB", "500.00", ts + DAY, "EMPLOYEE CONTRIBUTION")],
        );
        import_account_set(&conn, &set, true).await.unwrap();

        let mut cands = duplicate_candidates(&conn).await.unwrap();
        cands.sort_by(|a, b| a.account_id.cmp(&b.account_id));
        assert_eq!(cands.len(), 2);
        assert_eq!(cands[0].account_id, checking);
        assert_eq!(cands[0].bucket, "net_deposit");
        assert_eq!(cands[1].account_id, k401);
        assert_eq!(cands[1].bucket, "contribution");
    }

    #[tokio::test]
    async fn duplicate_candidates_pairs_greedily_one_to_one() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;

        let ts = 1_700_000_000;
        // Two manual rows that both match the single imported one; the
        // closer-dated row wins and the other is not paired at all.
        let same_day =
            insert_local_txn(&conn, acc, "expense", 20.0, &ts_to_date(ts), "gas", "manual", None)
                .await;
        insert_local_txn(
            &conn, acc, "expense", 20.0, &ts_to_date(ts + 2 * DAY), "gas again", "csv", None,
        )
        .await;

        let set = demo_set("1500.00", ts, vec![demo_txn("TRN-1", "-20.00", ts, "SHELL OIL")]);
        import_account_set(&conn, &set, true).await.unwrap();

        let cands = duplicate_candidates(&conn).await.unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].other_txn_id, same_day);
        assert_eq!(cands[0].other_import_source, "manual");
    }

    #[tokio::test]
    async fn transfer_pair_collapses_and_flags_contribution() {
        let conn = setup_db().await;
        let checking = insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let brokerage = insert_account(&conn, "Brokerage", "brokerage", "ACT-2").await;

        let ts = 1_700_000_000;
        let set = demo_set2(
            vec![demo_txn("TRN-OUT", "-500.00", ts, "TRANSFER TO VANGUARD")],
            // Destination reports the deposit a day later with unrelated wording.
            vec![demo_txn("TRN-IN", "500.00", ts + DAY, "ACH ELECTRONIC FUNDS")],
        );
        let (summary, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(summary.transactions_added, 2);
        assert_eq!(summary.transfers_detected, 1);

        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 1, "pair should collapse to a single row");
        let t = &txns[0];
        assert_eq!(t.0, checking); // source side survives
        assert_eq!(t.1, Some(brokerage));
        assert_eq!(t.2, 500.0);
        assert_eq!(t.3, "transfer");
        assert!(t.4, "transfer into an investment account is a contribution");
        assert_eq!(t.5.as_deref(), Some("TRN-OUT"));
        assert_eq!(t.6.as_deref(), Some("TRN-IN"));

        // Re-import (overlap window): the source dedupes on simplefin_id, the
        // deleted counterpart is blocked by simplefin_counterpart_id.
        let (again, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(again.transactions_added, 0);
        assert_eq!(again.transfers_detected, 0);
        assert_eq!(read_all_txns(&conn).await.len(), 1);
    }

    #[tokio::test]
    async fn transfer_to_non_investment_is_not_a_contribution() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "ACT-1").await;
        insert_account(&conn, "Savings", "savings", "ACT-2").await;

        let ts = 1_700_000_000;
        let set = demo_set2(
            vec![demo_txn("TRN-OUT", "-250.00", ts, "ONLINE TRANSFER")],
            vec![demo_txn("TRN-IN", "250.00", ts, "TRANSFER FROM CHECKING")],
        );
        let (summary, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(summary.transfers_detected, 1);
        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].3, "transfer");
        assert!(!txns[0].4, "asset-to-asset move is not a contribution");
    }

    #[tokio::test]
    async fn investment_to_investment_transfer_is_not_a_contribution() {
        let conn = setup_db().await;
        insert_account(&conn, "Brokerage", "brokerage", "ACT-1").await;
        insert_account(&conn, "Roth IRA", "roth_ira", "ACT-2").await;

        let ts = 1_700_000_000;
        let set = demo_set2(
            vec![demo_txn("TRN-OUT", "-7000.00", ts, "WITHDRAWAL")],
            vec![demo_txn("TRN-IN", "7000.00", ts + DAY, "CONTRIBUTION")],
        );
        let (summary, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(summary.transfers_detected, 1);
        let txns = read_all_txns(&conn).await;
        assert_eq!(txns[0].3, "transfer");
        assert!(!txns[0].4, "rollover between investment accounts is not new principal");
    }

    #[tokio::test]
    async fn unrelated_rows_do_not_collapse() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "ACT-1").await;
        insert_account(&conn, "Savings", "savings", "ACT-2").await;

        let ts = 1_700_000_000;
        let set = demo_set2(
            vec![
                // Amount differs from every income on the other side.
                demo_txn("TRN-A", "-99.99", ts, "GROCERIES"),
                // Amount matches but the date is outside the ±3 day window.
                demo_txn("TRN-B", "-500.00", ts, "RENT"),
            ],
            vec![demo_txn("TRN-C", "500.00", ts + 5 * DAY, "PAYROLL")],
        );
        let (summary, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(summary.transfers_detected, 0);
        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 3);
        assert!(txns.iter().all(|t| t.3 != "transfer"));
    }

    #[tokio::test]
    async fn counterpart_arriving_on_a_later_sync_still_collapses() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "ACT-1").await;
        insert_account(&conn, "Savings", "savings", "ACT-2").await;

        let ts = 1_700_000_000;
        // First sync: only the source side has posted.
        let set1 = demo_set2(vec![demo_txn("TRN-OUT", "-300.00", ts, "TRANSFER OUT")], vec![]);
        let (s1, _) = import_account_set(&conn, &set1, true).await.unwrap();
        assert_eq!(s1.transfers_detected, 0);
        assert_eq!(read_all_txns(&conn).await.len(), 1);

        // Next sync: the destination side arrives (plus the source re-fetched
        // via the overlap window).
        let set2 = demo_set2(
            vec![demo_txn("TRN-OUT", "-300.00", ts, "TRANSFER OUT")],
            vec![demo_txn("TRN-IN", "300.00", ts + 2 * DAY, "DEPOSIT")],
        );
        let (s2, _) = import_account_set(&conn, &set2, true).await.unwrap();
        assert_eq!(s2.transactions_added, 1);
        assert_eq!(s2.transfers_detected, 1);
        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].3, "transfer");
    }

    #[tokio::test]
    async fn counterpart_absorbs_into_user_converted_transfer() {
        let conn = setup_db().await;
        let checking = insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let mortgage = insert_account(&conn, "Mortgage", "loan", "ACT-2").await;

        let ts = 1_700_000_000;
        // First sync: only the mortgage side has posted — lands as income.
        let set1 = demo_set2(vec![], vec![demo_txn("TRN-MORT", "199.59", ts, "PAYMENT")]);
        import_account_set(&conn, &set1, true).await.unwrap();

        // User converts the row to a transfer by hand instead of waiting for
        // the counterpart (same columns update_transaction touches).
        conn.execute(
            "UPDATE txn SET type = 'transfer', account_id = ?1, transfer_account_id = ?2, \
             category = 'uncategorized' WHERE simplefin_id = 'TRN-MORT'",
            params![checking, mortgage],
        )
        .await
        .unwrap();

        // Next sync: the checking side posts. It must be absorbed, not kept
        // as a duplicate expense.
        let set2 = demo_set2(vec![demo_txn("TRN-PNC", "-199.59", ts + DAY, "ONLINE PMT")], vec![]);
        let (s2, _) = import_account_set(&conn, &set2, true).await.unwrap();
        assert_eq!(s2.transfers_detected, 1);

        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 1, "counterpart should be absorbed into the transfer");
        let t = &txns[0];
        assert_eq!(t.0, checking);
        assert_eq!(t.1, Some(mortgage));
        assert_eq!(t.3, "transfer");
        assert_eq!(t.5.as_deref(), Some("TRN-MORT"));
        assert_eq!(t.6.as_deref(), Some("TRN-PNC"));

        // Re-import (overlap window): the absorbed id is blocked by
        // simplefin_counterpart_id, and the transfer is claimed exactly once.
        let (again, _) = import_account_set(&conn, &set2, true).await.unwrap();
        assert_eq!(again.transactions_added, 0);
        assert_eq!(again.transfers_detected, 0);
        assert_eq!(read_all_txns(&conn).await.len(), 1);
    }

    #[tokio::test]
    async fn absorb_requires_matching_orientation_and_amount() {
        let conn = setup_db().await;
        let checking = insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let mortgage = insert_account(&conn, "Mortgage", "loan", "ACT-2").await;

        let ts = 1_700_000_000;
        let set1 = demo_set2(vec![], vec![demo_txn("TRN-MORT", "199.59", ts, "PAYMENT")]);
        import_account_set(&conn, &set1, true).await.unwrap();
        conn.execute(
            "UPDATE txn SET type = 'transfer', account_id = ?1, transfer_account_id = ?2, \
             category = 'uncategorized' WHERE simplefin_id = 'TRN-MORT'",
            params![checking, mortgage],
        )
        .await
        .unwrap();

        // An income on the source account (wrong orientation) and an expense
        // with a different amount: neither may be absorbed.
        let set2 = demo_set2(
            vec![
                demo_txn("TRN-DEP", "199.59", ts + DAY, "DEPOSIT"),
                demo_txn("TRN-OTHER", "-210.00", ts + DAY, "GROCERIES"),
            ],
            vec![],
        );
        let (s2, _) = import_account_set(&conn, &set2, true).await.unwrap();
        assert_eq!(s2.transfers_detected, 0);
        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 3);
        let transfer = txns.iter().find(|t| t.3 == "transfer").unwrap();
        assert_eq!(transfer.6, None, "transfer must not claim a mismatched counterpart");
    }

    #[tokio::test]
    async fn greedy_matching_pairs_each_row_once() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "ACT-1").await;
        insert_account(&conn, "Savings", "savings", "ACT-2").await;

        let ts = 1_700_000_000;
        // Two same-amount expenses but only one matching income: exactly one
        // pair collapses, the other expense stays an expense.
        let set = demo_set2(
            vec![
                demo_txn("TRN-A", "-100.00", ts, "TRANSFER"),
                demo_txn("TRN-B", "-100.00", ts + DAY, "STORE PURCHASE"),
            ],
            vec![demo_txn("TRN-IN", "100.00", ts, "DEPOSIT")],
        );
        let (summary, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(summary.transfers_detected, 1);
        let txns = read_all_txns(&conn).await;
        assert_eq!(txns.len(), 2);
        // The closest-dated expense (TRN-A, same day) won the pairing.
        let transfer = txns.iter().find(|t| t.3 == "transfer").unwrap();
        assert_eq!(transfer.5.as_deref(), Some("TRN-A"));
        assert!(txns.iter().any(|t| t.3 == "expense" && t.5.as_deref() == Some("TRN-B")));
    }

    #[tokio::test]
    async fn import_flags_dedupes_and_maps_signs() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;

        let ts = 1_700_000_000; // fixed instant
        let set = demo_set(
            "1500.00",
            ts,
            vec![
                demo_txn("TRN-1", "-42.50", ts, "STARBUCKS #123"),
                demo_txn("TRN-2", "2000.00", ts, "PAYROLL ACME"),
            ],
        );

        let (summary, cache) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(summary.accounts_synced, 1);
        assert_eq!(summary.transactions_added, 2);
        assert_eq!(summary.snapshots_added, 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].name, "Demo Checking");

        // Re-import (overlapping window): everything dedupes.
        let (again, _) = import_account_set(&conn, &set, true).await.unwrap();
        assert_eq!(again.transactions_added, 0);
        assert_eq!(again.snapshots_added, 0);

        // Signs, flags, and provenance.
        let mut rows = conn
            .query(
                "SELECT amount, type, import_source, simplefin_id, category \
                 FROM txn ORDER BY simplefin_id",
                (),
            )
            .await
            .unwrap();
        let r1 = rows.next().await.unwrap().unwrap();
        assert_eq!(r1.get::<f64>(0).unwrap(), 42.50);
        assert_eq!(r1.get::<String>(1).unwrap(), "expense");
        assert_eq!(r1.get::<String>(2).unwrap(), "simplefin");
        assert_eq!(r1.get::<String>(3).unwrap(), "TRN-1");
        // "STARBUCKS #123" hits the seeded 'starbucks' → discretionary rule.
        assert_eq!(r1.get::<String>(4).unwrap(), "discretionary");
        let r2 = rows.next().await.unwrap().unwrap();
        assert_eq!(r2.get::<f64>(0).unwrap(), 2000.00);
        assert_eq!(r2.get::<String>(1).unwrap(), "income");

        // Snapshot provenance flag.
        let mut snaps = conn
            .query(
                "SELECT balance, source FROM account_balance WHERE account_id = ?1",
                params![acc],
            )
            .await
            .unwrap();
        let s = snaps.next().await.unwrap().unwrap();
        assert_eq!(s.get::<f64>(0).unwrap(), 1500.0);
        assert_eq!(s.get::<String>(1).unwrap(), "simplefin");
    }

    #[tokio::test]
    async fn liability_balance_sign_is_flipped() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Card", "liability", "ACT-1").await;
        let set = demo_set("-543.21", 1_700_000_000, vec![]);
        import_account_set(&conn, &set, true).await.unwrap();
        let mut snaps = conn
            .query(
                "SELECT balance FROM account_balance WHERE account_id = ?1",
                params![acc],
            )
            .await
            .unwrap();
        let s = snaps.next().await.unwrap().unwrap();
        // Bank reports -543.21 owed; the app stores debt as positive.
        assert_eq!(s.get::<f64>(0).unwrap(), 543.21);
    }

    #[tokio::test]
    async fn same_day_rereport_updates_snapshot_in_place() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let ts = 1_700_000_000;
        import_account_set(&conn, &demo_set("100.00", ts, vec![]), true).await.unwrap();
        // Same balance-date, new value (e.g. later in the same day).
        let (s2, _) = import_account_set(&conn, &demo_set("150.00", ts, vec![]), true).await.unwrap();
        assert_eq!(s2.snapshots_added, 0);
        let mut snaps = conn
            .query(
                "SELECT COUNT(*), MAX(balance) FROM account_balance WHERE account_id = ?1",
                params![acc],
            )
            .await
            .unwrap();
        let r = snaps.next().await.unwrap().unwrap();
        assert_eq!(r.get::<i64>(0).unwrap(), 1);
        assert_eq!(r.get::<f64>(1).unwrap(), 150.0);
    }

    #[tokio::test]
    async fn pending_and_unlinked_are_skipped() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "SOMETHING-ELSE").await;
        let mut pending = demo_txn("TRN-P", "-5.00", 1_700_000_000, "PENDING CHARGE");
        pending.pending = Some(true);
        let set = demo_set("100.00", 1_700_000_000, vec![pending]);
        let (summary, cache) = import_account_set(&conn, &set, true).await.unwrap();
        // ACT-1 isn't linked to any local account: nothing imports, but the
        // account still appears in the cache for the linking UI.
        assert_eq!(summary.accounts_synced, 0);
        assert_eq!(summary.transactions_added, 0);
        assert_eq!(summary.snapshots_added, 0);
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn pending_rows_cache_and_clear_when_posted() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let ts = 1_700_000_000;

        let mut pending = demo_txn("TRN-P", "-12.34", ts, "PENDING COFFEE");
        pending.pending = Some(true);
        let (s1, _) =
            import_account_set(&conn, &demo_set("100.00", ts, vec![pending]), true).await.unwrap();
        assert!(s1.pending_changed);
        assert_eq!(s1.transactions_added, 0, "pending must not enter the ledger");

        let mut rows = conn
            .query(
                "SELECT account_id, amount, type, simplefin_id FROM simplefin_pending_txn",
                (),
            )
            .await
            .unwrap();
        let r = rows.next().await.unwrap().unwrap();
        assert_eq!(r.get::<i32>(0).unwrap(), acc);
        assert_eq!(r.get::<f64>(1).unwrap(), 12.34);
        assert_eq!(r.get::<String>(2).unwrap(), "expense");
        assert_eq!(r.get::<String>(3).unwrap(), "TRN-P");
        assert!(rows.next().await.unwrap().is_none());
        drop(rows);

        // Next sync: the charge has posted (same id kept by this bridge). The
        // cache empties and the ledger gains the posted row exactly once.
        let (s2, _) = import_account_set(
            &conn,
            &demo_set("87.66", ts + DAY, vec![demo_txn("TRN-P", "-12.34", ts + DAY, "COFFEE")]),
            true,
        )
        .await
        .unwrap();
        assert!(s2.pending_changed);
        assert_eq!(s2.transactions_added, 1);
        let mut count = conn
            .query("SELECT COUNT(*) FROM simplefin_pending_txn", ())
            .await
            .unwrap();
        assert_eq!(count.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 0);

        // Idempotent re-import with no pending: nothing changed.
        let (s3, _) = import_account_set(
            &conn,
            &demo_set("87.66", ts + DAY, vec![demo_txn("TRN-P", "-12.34", ts + DAY, "COFFEE")]),
            true,
        )
        .await
        .unwrap();
        assert!(!s3.pending_changed);
    }

    #[tokio::test]
    async fn backfill_import_preserves_pending_cache_and_clears_posted_rows() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let ts = 1_700_000_000;

        // Regular sync caches two pending charges.
        let mut p1 = demo_txn("TRN-P1", "-10.00", ts, "PENDING ONE");
        p1.pending = Some(true);
        let mut p2 = demo_txn("TRN-P2", "-20.00", ts, "PENDING TWO");
        p2.pending = Some(true);
        import_account_set(&conn, &demo_set("100.00", ts, vec![p1, p2]), true).await.unwrap();

        // Backfill (rebuild_pending = false) importing the posted form of one
        // of them: that row leaves the cache, the other survives the import.
        let (s, _) = import_account_set(
            &conn,
            &demo_set("90.00", ts + DAY, vec![demo_txn("TRN-P1", "-10.00", ts, "POSTED ONE")]),
            false,
        )
        .await
        .unwrap();
        assert_eq!(s.transactions_added, 1);
        assert!(s.pending_changed);

        let mut rows = conn
            .query("SELECT simplefin_id FROM simplefin_pending_txn", ())
            .await
            .unwrap();
        let r = rows.next().await.unwrap().unwrap();
        assert_eq!(r.get::<String>(0).unwrap(), "TRN-P2");
        assert!(rows.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn pending_row_already_posted_under_same_id_is_not_cached() {
        let conn = setup_db().await;
        insert_account(&conn, "Checking", "checking", "ACT-1").await;
        let ts = 1_700_000_000;

        // Posted import first…
        import_account_set(
            &conn,
            &demo_set("100.00", ts, vec![demo_txn("TRN-1", "-5.00", ts, "CHARGE")]),
            true,
        )
        .await
        .unwrap();
        // …then a stale fetch still marks the same id as pending: guarded out.
        let mut stale = demo_txn("TRN-1", "-5.00", ts, "CHARGE");
        stale.pending = Some(true);
        import_account_set(&conn, &demo_set("100.00", ts, vec![stale]), true).await.unwrap();
        let mut count = conn
            .query("SELECT COUNT(*) FROM simplefin_pending_txn", ())
            .await
            .unwrap();
        assert_eq!(count.next().await.unwrap().unwrap().get::<i64>(0).unwrap(), 0);
    }

    #[tokio::test]
    async fn simplefin_snapshot_reanchors_later_generated_snapshots() {
        let conn = setup_db().await;
        let acc = insert_account(&conn, "Checking", "checking", "ACT-1").await;

        // A user-entered transaction with a generated snapshot dated later
        // than the incoming simplefin balance-date.
        let t = crate::commands::transactions::NewTransaction {
            account_id: acc,
            transfer_account_id: None,
            amount: 100.0,
            description: "income".into(),
            date: "2099-01-15".into(),
            r#type: "income".into(),
            category: "uncategorized".into(),
            is_contribution: false,
            is_withdrawal: false,
            is_refund: false,
            import_source: "manual".into(),
            update_balance: true,
            created_at: "2099-01-15T00:00:00".into(),
        };
        crate::commands::transactions::create_transaction_synced(&conn, &t)
            .await
            .unwrap();

        // SimpleFIN reports 1000 today (long before 2099): the later generated
        // snapshot must re-anchor to 1000 + 100.
        let set = demo_set("1000.00", 1_700_000_000, vec![]);
        import_account_set(&conn, &set, true).await.unwrap();

        let mut rows = conn
            .query(
                "SELECT balance FROM account_balance WHERE account_id = ?1 \
                 ORDER BY recorded_at DESC LIMIT 1",
                params![acc],
            )
            .await
            .unwrap();
        let r = rows.next().await.unwrap().unwrap();
        assert_eq!(r.get::<f64>(0).unwrap(), 1100.0);
    }
}
