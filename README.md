# TrackMyFI

A local-first desktop app for personal FIRE (Financial Independence, Retire Early) tracking.
Your data lives in a local libSQL/SQLite database on your machine — no account required and
fully usable offline. Optional, opt-in cloud sync (via [Turso](https://turso.tech)) lets you
reconcile across machines.

Built with [Tauri](https://tauri.app) (Rust backend) and Vue 3 + TypeScript (Vite frontend).

## Development

```bash
npm run tauri dev   # run the full desktop app (Vite dev server + Rust backend)
npm run dev         # frontend only, no Rust backend (UI-only work)
npm run build       # type-check + build the frontend
npm test            # run frontend unit tests
```

Debug builds (`tauri dev`) use an isolated `dev/` database directory, so you can never corrupt
release data during development.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Cloud Sync (optional)

TrackMyFI is local-first and works fully offline — your data lives in a local database on
your machine and you never need an account. Cloud sync is entirely optional and opt-in; turn
it on only if you want to keep a copy in your own cloud database and reconcile your data
across machines.

### Enabling sync

1. Create a free [Turso](https://turso.tech) database, either:
   - in the browser at [turso.tech](https://turso.tech), or
   - with the Turso CLI: `turso db create trackmyfi`
2. Copy the database's **Database URL** (starts with `libsql://`) and create an **auth token**.
3. In the app, open **Settings → Cloud Sync**, paste the URL and token, and enable sync.
4. **Restart the app** to start syncing.

When you enable sync, your existing local data is uploaded to the new (empty) cloud database.
Your previous local database file is kept on disk as a backup and is never deleted automatically.

### How your data is protected

- The auth token is stored in your operating system's keychain (Keychain on macOS,
  Credential Manager on Windows, the Secret Service on Linux).
- The local database file is protected by your operating system's full-disk encryption
  (FileVault on macOS, BitLocker on Windows). The app itself does not encrypt the database
  file, and Turso does not provide local at-rest encryption — enable your OS full-disk
  encryption if you want the file encrypted at rest.
- Data syncs to Turso over an encrypted (TLS) connection.

## Releasing

Updates are delivered through the in-app update popover: GitHub Actions builds and signs a
new version, publishes it to GitHub Releases, and the running app polls that release and
offers a one-click download + install.

### Cutting a release

```bash
npm run release -- v0.2.0   # the leading "v" is optional
```

This command:

1. **Validates** the version (semver) and aborts if that tag already exists or the version is unchanged.
2. **Pre-flight builds** locally (`npm run build` + `cargo check`) so a broken build never gets tagged. Bypass with `npm run release -- v0.2.0 --skip-checks`.
3. **Bumps the version** in all four files that carry it: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock`.
4. **Commits** just those files as `chore: release v0.2.0` — any other uncommitted work is left untouched.
5. **Tags** `v0.2.0` and **pushes** the branch + tag.

Pushing the tag triggers `.github/workflows/release.yml`, which builds, signs, and publishes
the macOS app plus the `latest.json` update manifest to GitHub Releases.

> **First install needs one Gatekeeper override.** Because the app is self-signed rather than
> notarized by Apple, macOS blocks a manually downloaded build the first time ("Apple could not
> verify… is free of malware"). Click **Done** (not *Move to Trash*), then clear the quarantine
> flag and open it:
>
> ```bash
> xattr -dr com.apple.quarantine ~/Downloads/TrackMyFI_*_universal.dmg
> open ~/Downloads/TrackMyFI_*_universal.dmg
> # drag TrackMyFI to /Applications, then if launching is still blocked:
> xattr -dr com.apple.quarantine /Applications/TrackMyFI.app
> ```
>
> (GUI alternative: after clicking Done, go to System Settings → **Privacy & Security** →
> **Open Anyway**.) This is only ever needed for a manually downloaded `.dmg` — **in-app
> auto-updates are not affected**, since the running app applies them in place without
> re-triggering Gatekeeper. (Notarizing with an Apple Developer ID would remove this prompt
> entirely.)

### Signing secrets (one-time, on GitHub)

CI signs everything using **repository secrets**, so the signing keys do not need to live on
the machine you release from:

| Secret | Purpose |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | minisign key that signs the update artifacts (the app verifies these against the public key in `tauri.conf.json`). |
| `MACOS_CERTIFICATE` / `MACOS_CERTIFICATE_PASSWORD` | base64 of the self-signed code-signing `.p12` and its export password. |

### Working from another computer

You do **not** need the signing certificate on a machine just to cut a release —
`npm run release` only bumps, commits, tags, and pushes; all signing happens in CI using the
secrets above. You only need the certificate locally if you want to produce a signed build
directly with `npm run tauri build` (plain `npm run tauri dev` does not sign and needs nothing).

To put the certificate on another Mac:

1. On the machine that already has it, export it: Keychain Access → **login** keychain →
   **My Certificates** → right-click **TrackMyFI Self-Signed** → **Export…** → save as `.p12`
   with a password. (Exporting from *My Certificates* ensures the private key is included.)
2. Copy the `.p12` to the other Mac.
3. Import it into the login keychain:
   ```bash
   security import TrackMyFI-Self-Signed.p12 \
     -k ~/Library/Keychains/login.keychain-db \
     -P 'YOUR_EXPORT_PASSWORD' \
     -T /usr/bin/codesign
   ```
   (Or simply double-click the `.p12` in Finder.)
4. Verify it can be used for signing:
   ```bash
   security find-identity -p codesigning -v | grep "TrackMyFI Self-Signed"
   ```

The identity name must stay **`TrackMyFI Self-Signed`** so it matches `bundle.macOS.signingIdentity`
in `src-tauri/tauri.conf.json`.
