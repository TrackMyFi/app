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
//! Rate limits: SimpleFIN asks clients to poll roughly once a day. The
//! scheduler in `lib.rs` syncs 24h after the last success, retrying no sooner
//! than 6h after a failed attempt. "Sync now" is always allowed (user action).
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

/// Sync 24h after the last successful sync (SimpleFIN's ~once-a-day guidance).
const SYNC_AFTER_SUCCESS_HOURS: i64 = 24;
/// After a failed attempt, wait this long before retrying automatically.
const RETRY_AFTER_FAILURE_HOURS: i64 = 6;
/// Re-fetch this many days before the last success so nothing on the boundary
/// is missed; the unique simplefin_id index absorbs the duplicates.
const OVERLAP_DAYS: i64 = 3;
/// Transaction lookback for the very first sync after connecting.
const FIRST_SYNC_LOOKBACK_DAYS: i64 = 90;
/// How often the background scheduler re-checks whether a sync is due.
pub const SCHEDULER_TICK_SECS: u64 = 1800; // 30 minutes

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
    pub bridge_errors: Vec<String>,
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

/// Whether an automatic sync is due. Pure so it's testable: 24h after the last
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
    let attempt_ok = match last_attempt {
        Some(t) => now - t >= chrono::Duration::hours(RETRY_AFTER_FAILURE_HOURS),
        None => true,
    };
    success_due && attempt_ok
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
async fn fetch_accounts(
    access_url: &str,
    start_date: Option<i64>,
) -> Result<SfinAccountSet, String> {
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
    resp.json::<SfinAccountSet>()
        .await
        .map_err(|e| format!("Could not parse the SimpleFIN response: {e}"))
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
async fn import_account_set(
    conn: &Connection,
    set: &SfinAccountSet,
) -> Result<(SimpleFinSyncSummary, Vec<SimpleFinRemoteAccount>), String> {
    // simplefin_id → local account (id + liability flag, for balance sign).
    let mut linked: std::collections::HashMap<String, LinkedAccount> =
        std::collections::HashMap::new();
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

    // Category rules, first-match-wins in id order (same as the CSV importer).
    let mut rules: Vec<(String, String)> = Vec::new();
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

    let mut summary = SimpleFinSyncSummary::default();
    summary.bridge_errors = set.errors.clone();
    let mut cache: Vec<SimpleFinRemoteAccount> = Vec::new();
    // Accounts whose snapshot chain may need re-anchoring, with the earliest
    // date this sync touched per account.
    let mut reproject: Vec<(i32, String)> = Vec::new();
    let created_at = now_iso();

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
        let mut existing = conn
            .query(
                "SELECT id, balance FROM account_balance \
                 WHERE account_id = ?1 AND source = 'simplefin' AND recorded_at = ?2 \
                 ORDER BY id DESC LIMIT 1",
                params![link.account_id, balance_date.clone()],
            )
            .await
            .map_err(|e| e.to_string())?;
        match existing.next().await.map_err(|e| e.to_string())? {
            Some(r) => {
                let snap_id: i32 = r.get(0).map_err(|e| e.to_string())?;
                let old: f64 = r.get(1).map_err(|e| e.to_string())?;
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

        // Transactions. Pending ones are skipped — they mutate or vanish, and
        // the next daily sync picks them up once posted.
        for t in &acct.transactions {
            if t.pending == Some(true) {
                continue;
            }
            let Some(signed) = t.amount.as_f64() else { continue };
            let ts = if t.posted > 0 { t.posted } else { t.transacted_at.unwrap_or(0) };
            if ts <= 0 {
                continue;
            }
            let date = ts_to_date(ts);
            let (ty, amount) =
                if signed < 0.0 { ("expense", -signed) } else { ("income", signed) };
            let description = [t.payee.as_deref(), t.description.as_deref(), t.memo.as_deref()]
                .into_iter()
                .flatten()
                .map(str::trim)
                .find(|s| !s.is_empty())
                .unwrap_or("SimpleFIN transaction")
                .to_string();
            let category = match_category(&rules, &description);
            let vendor_cat = vendor_category(t);

            // No generated balance snapshots for synced transactions — the
            // bridge's own balance snapshot is the authoritative anchor.
            let inserted = conn
                .execute(
                    "INSERT OR IGNORE INTO txn (account_id, transfer_account_id, amount, \
                     description, date, type, category, is_contribution, is_withdrawal, \
                     import_source, generated_balance_id, generated_balance_to_id, \
                     vendor_category, simplefin_id, created_at, updated_at) \
                     VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 0, 0, 'simplefin', NULL, NULL, \
                     ?7, ?8, ?9, ?9)",
                    params![
                        link.account_id,
                        amount,
                        description,
                        date,
                        ty,
                        category,
                        vendor_cat,
                        t.id.clone(),
                        created_at.clone()
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

    Ok((summary, cache))
}

// ---- sync orchestration ----

/// The single funnel all SimpleFIN sync triggers call.
pub async fn run_sync(app: &AppHandle) -> Result<SimpleFinSyncSummary, String> {
    let shared = app.state::<SimpleFinShared>();
    let _guard = shared.lock.lock().await; // serialize scheduler vs. manual click

    let access_url = access_url_get()?
        .ok_or_else(|| "SimpleFIN is not connected on this device.".to_string())?;
    let db = app.state::<Db>();
    let conn = db.conn().await?;

    let state = read_state(&conn).await?;
    set_state(&conn, "last_attempt_at", Some(&now_iso())).await?;

    // Transaction window: always overlap back past the last SUCCESS, so a
    // connection that was broken for days or weeks backfills the whole gap.
    let start_ts = match state.last_success_at.as_deref().and_then(parse_iso) {
        Some(t) => t.timestamp() - OVERLAP_DAYS * 86_400,
        None => Utc::now().timestamp() - FIRST_SYNC_LOOKBACK_DAYS * 86_400,
    };

    let result: Result<SimpleFinSyncSummary, String> = async {
        let set = fetch_accounts(&access_url, Some(start_ts)).await?;
        let (summary, cache) = import_account_set(&conn, &set).await?;
        let accounts_json = serde_json::to_string(&cache).map_err(|e| e.to_string())?;
        let bridge_json = serde_json::to_string(&set.errors).map_err(|e| e.to_string())?;
        set_state(&conn, "accounts_json", Some(&accounts_json)).await?;
        set_state(&conn, "bridge_errors", Some(&bridge_json)).await?;
        set_state(&conn, "last_success_at", Some(&now_iso())).await?;
        set_state(&conn, "last_error", None).await?;
        Ok(summary)
    }
    .await;

    if let Err(e) = &result {
        set_state(&conn, "last_error", Some(e)).await?;
    }

    // Tell the frontend. Data pages remount only when something changed.
    if let Ok(status) = build_status(&conn).await {
        let _ = app.emit("simplefin-status", status);
    }
    if let Ok(s) = &result {
        if s.transactions_added > 0 || s.snapshots_added > 0 {
            let _ = app.emit("data-refreshed", ());
        }
    }

    result
}

/// Scheduler entry point: sync only when due. Errors are recorded in the state
/// row by `run_sync`; the scheduler itself never fails.
pub async fn maybe_sync(app: &AppHandle) {
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
    fn sync_due_backs_off_after_failures() {
        let now = Utc::now();
        let h = chrono::Duration::hours;
        // Never synced → due.
        assert!(sync_due(now, None, None));
        // Succeeded 2h ago → not due.
        assert!(!sync_due(now, Some(now - h(2)), Some(now - h(2))));
        // Succeeded 25h ago, no recent attempt → due.
        assert!(sync_due(now, Some(now - h(25)), Some(now - h(25))));
        // Succeeded 25h ago but a (failed) attempt 2h ago → back off.
        assert!(!sync_due(now, Some(now - h(25)), Some(now - h(2))));
        // ...and retry once the attempt is 6h old.
        assert!(sync_due(now, Some(now - h(30)), Some(now - h(7))));
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

        let (summary, cache) = import_account_set(&conn, &set).await.unwrap();
        assert_eq!(summary.accounts_synced, 1);
        assert_eq!(summary.transactions_added, 2);
        assert_eq!(summary.snapshots_added, 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[0].name, "Demo Checking");

        // Re-import (overlapping window): everything dedupes.
        let (again, _) = import_account_set(&conn, &set).await.unwrap();
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
        import_account_set(&conn, &set).await.unwrap();
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
        import_account_set(&conn, &demo_set("100.00", ts, vec![])).await.unwrap();
        // Same balance-date, new value (e.g. later in the same day).
        let (s2, _) = import_account_set(&conn, &demo_set("150.00", ts, vec![])).await.unwrap();
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
        let (summary, cache) = import_account_set(&conn, &set).await.unwrap();
        // ACT-1 isn't linked to any local account: nothing imports, but the
        // account still appears in the cache for the linking UI.
        assert_eq!(summary.accounts_synced, 0);
        assert_eq!(summary.transactions_added, 0);
        assert_eq!(summary.snapshots_added, 0);
        assert_eq!(cache.len(), 1);
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
        import_account_set(&conn, &set).await.unwrap();

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
