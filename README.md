# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

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
