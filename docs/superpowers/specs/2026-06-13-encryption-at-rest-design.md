# Encryption-at-Rest: Decision Record

**Date:** 2026-06-13
**Status:** CLOSED — Not implemented. Encryption-at-rest is delegated to OS full-disk encryption.

## Decision

Do **not** build app-level database encryption. Treat encryption-at-rest as **satisfied by OS
full-disk encryption** (FileVault on macOS, BitLocker on Windows). No new code, no new
dependencies, no data-layer changes.

## Context

We set out to add app-managed encryption of the local libSQL file, with the key in the OS
keychain and a BIP39 24-word recovery phrase for key loss (full design was drafted and approved
in brainstorming). Implementation surfaced two facts that changed the calculus:

1. **libSQL's `encryption` feature requires CMake.** It builds a cipher-enabled SQLite
   (`SQLite3MultipleCiphers`, AES-256) from C via CMake, which isn't installed and would become a
   permanent build dependency for every dev machine, CI runner, and release build. The only
   cmake-free way to keep *transparent* whole-DB encryption is to migrate the data layer from
   libSQL to rusqlite + SQLCipher — which abandons libSQL and the planned Turso embedded-replica
   sync path.

2. **FileVault is already On.** The whole disk — including `trackmyfi.db` — is already encrypted
   at rest by the OS. The primary threat behind "encryption-at-rest" (a powered-off/stolen laptop,
   a copied `.db` file) is already covered.

Given (2), the marginal security value of app-level encryption is small: it would only help if the
`.db` were copied somewhere FileVault doesn't cover (an unencrypted external backup, a cloud-sync
folder), and with a transparently keychain-stored key even that gain is limited (anything running
as the logged-in user can read the key too). That is not worth the CMake dependency or a
libSQL→SQLCipher migration.

## Alternatives considered

- **libSQL `encryption` feature (original approved design)** — real transparent encryption, but
  requires CMake as a permanent build dependency. Rejected.
- **rusqlite + SQLCipher** — cmake-free transparent encryption, but migrates off libSQL and
  forecloses the Turso embedded-replica sync roadmap. Rejected.
- **Pure-Rust field encryption** — breaks the app's heavy SQL aggregation on `amount`/`date`
  (budget sums, contribution totals, balance recency); could only cover free-text labels. Rejected.
- **FileVault + lightweight startup check** that warns if full-disk encryption is off — considered;
  declined in favor of no new code.

## Note on Turso

Turso (deferred) does **not** provide local-file encryption — the embedded replica stays a plain
file on disk. Turso solves **durability/backup and multi-device sync**, and it would put financial
data in a third-party cloud (a shift from the app's local-first, no-server posture). It is a
separate future feature, not an encryption mechanism, and should be decided on its own merits.

## Consequences

- The design doc's architecture note now states encryption-at-rest is delegated to OS full-disk
  encryption.
- If app-level encryption is ever genuinely needed (e.g., distributing to users who can't be
  assumed to have full-disk encryption on), revisit the libSQL-`encryption`-via-CMake path — the
  full implementation design is preserved in this file's git history (commit on the
  `encryption-at-rest` branch, since deleted).
