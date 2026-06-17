# Import Mapping Chip Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add rename and delete actions to saved column mapping chips in the Import Wizard, change chips from green to gray by default, and flash green with "✓ Applied" for 1.75s when a mapping is applied.

**Architecture:** New Rust `update_import_mapping_cmd` handles rename persistence. The Vue component tracks `appliedMappingId` (applied flash) and `editingMappingId` / `editingMappingName` (inline rename) as refs. Each chip slot becomes a `div.group` that reveals pencil and × icon buttons on hover; when `editingMappingId` matches the chip, it renders an inline input instead.

**Tech Stack:** Rust / Tauri commands, libSQL, Vue 3 Composition API, Nuxt UI v3 (`UButton`, `UInput`), `@tauri-apps/plugin-dialog`

---

## Files

| File | Change |
|------|--------|
| `src-tauri/src/commands/import_mappings.rs` | Add `update_import_mapping` + `update_import_mapping_cmd` |
| `src-tauri/src/lib.rs` | Register `update_import_mapping_cmd` |
| `src/lib/api/importMappings.ts` | Export `updateImportMapping` |
| `src/components/ImportWizard.vue` | New refs + handlers + revised chip template (step 1 + step 2) |

---

### Task 1: Add Rust update command

**Files:**
- Modify: `src-tauri/src/commands/import_mappings.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `update_import_mapping` function and command**

Open `src-tauri/src/commands/import_mappings.rs`. Append after the `delete_import_mapping_cmd` block (after line 78):

```rust
pub async fn update_import_mapping(conn: &Connection, id: i32, name: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE import_mapping SET name = ?1 WHERE id = ?2",
        params![name, id],
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_import_mapping_cmd(
    db: State<'_, Db>,
    id: i32,
    name: String,
) -> Result<(), String> {
    let conn = db.conn().await?;
    update_import_mapping(&conn, id, &name).await
}
```

- [ ] **Step 2: Register the new command in lib.rs**

Open `src-tauri/src/lib.rs`. Find the block around line 67–69:

```rust
            commands::import_mappings::list_import_mappings_cmd,
            commands::import_mappings::create_import_mapping_cmd,
            commands::import_mappings::delete_import_mapping_cmd,
```

Replace it with:

```rust
            commands::import_mappings::list_import_mappings_cmd,
            commands::import_mappings::create_import_mapping_cmd,
            commands::import_mappings::update_import_mapping_cmd,
            commands::import_mappings::delete_import_mapping_cmd,
```

- [ ] **Step 3: Verify the Rust build compiles cleanly**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: ends with `Finished` and no `error` lines. Fix any compiler errors before continuing.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/import_mappings.rs src-tauri/src/lib.rs
git commit -m "feat: add update_import_mapping_cmd Rust command"
```

---

### Task 2: Add TypeScript API export

**Files:**
- Modify: `src/lib/api/importMappings.ts`

- [ ] **Step 1: Add `updateImportMapping` export**

Open `src/lib/api/importMappings.ts`. The current file ends with:

```ts
export const deleteImportMapping = (id: number) =>
  invoke<void>('delete_import_mapping_cmd', { id })
```

Append after it:

```ts
export const updateImportMapping = (id: number, name: string) =>
  invoke<void>('update_import_mapping_cmd', { id, name })
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/api/importMappings.ts
git commit -m "feat: export updateImportMapping TS API"
```

---

### Task 3: Add state refs and handlers to ImportWizard.vue

**Files:**
- Modify: `src/components/ImportWizard.vue`

The script section currently imports from `'../lib/api/importMappings'` as `* as mappingApi`. All new functions go through that namespace so no import change is needed for the API. However, `confirm` from `@tauri-apps/plugin-dialog` must be added — `window.confirm` is a no-op in the Tauri webview.

- [ ] **Step 1: Add `confirm` import**

Find the existing plugin-dialog import line (if any) or add after the existing Tauri import. The current imports include:

```ts
import { addBalance } from '../lib/api/accounts'
```

Add directly after the last import block (before the `const emit` line):

```ts
import { confirm } from '@tauri-apps/plugin-dialog'
```

- [ ] **Step 2: Add applied-animation refs**

Find this existing ref declaration block:

```ts
const newMappingName = ref('')
```

Add the following immediately after it:

```ts
const appliedMappingId = ref<number | null>(null)
let appliedTimer: ReturnType<typeof setTimeout> | null = null

const editingMappingId = ref<number | null>(null)
const editingMappingName = ref('')
```

- [ ] **Step 3: Update `applySavedMapping` to trigger the flash**

Find the existing `applySavedMapping` function:

```ts
function applySavedMapping(m: ImportMapping) {
  // Spread config.value first so any fields missing from older saved mappings
  // (e.g. amountMode, creditColumn added in later versions) fall back to the
  // current defaults rather than being dropped entirely.
  config.value = { ...config.value, ...JSON.parse(m.config) }
  toast.add({ title: `"${m.name}" mapping loaded`, color: 'success' })
}
```

Replace it with:

```ts
function applySavedMapping(m: ImportMapping) {
  config.value = { ...config.value, ...JSON.parse(m.config) }
  if (appliedTimer !== null) clearTimeout(appliedTimer)
  appliedMappingId.value = m.id
  appliedTimer = setTimeout(() => {
    appliedMappingId.value = null
    appliedTimer = null
  }, 1750)
}
```

- [ ] **Step 4: Add rename handlers**

Add the following functions after `applySavedMapping`:

```ts
function startRename(m: ImportMapping) {
  editingMappingId.value = m.id
  editingMappingName.value = m.name
}

function cancelRename() {
  editingMappingId.value = null
  editingMappingName.value = ''
}

async function saveRename(m: ImportMapping) {
  const trimmed = editingMappingName.value.trim()
  if (!trimmed) {
    cancelRename()
    return
  }
  try {
    await mappingApi.updateImportMapping(m.id, trimmed)
    savedMappings.value = await mappingApi.listImportMappings()
  } catch {
    toast.add({ title: 'Failed to rename mapping', color: 'error' })
  }
  cancelRename()
}

async function deleteMapping(m: ImportMapping) {
  const ok = await confirm(`Delete "${m.name}"?`, { title: 'Delete mapping', kind: 'warning' })
  if (!ok) return
  try {
    await mappingApi.deleteImportMapping(m.id)
    savedMappings.value = await mappingApi.listImportMappings()
  } catch {
    toast.add({ title: 'Failed to delete mapping', color: 'error' })
  }
}
```

- [ ] **Step 5: Verify TypeScript compiles**

```bash
npx tsc --noEmit 2>&1 | head -30
```

Expected: no output (no errors). Fix any type errors before continuing.

- [ ] **Step 6: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: add mapping chip state, applied flash, rename/delete handlers"
```

---

### Task 4: Update chip template — Step 1

**Files:**
- Modify: `src/components/ImportWizard.vue` (template, step 1 section)

The current step-1 chip list (around line 349–356) looks like:

```html
<div v-if="savedMappings.length" class="border-t border-default pt-4 space-y-2">
  <p class="text-xs text-muted">Apply a saved column mapping after upload:</p>
  <div class="flex flex-wrap gap-1.5">
    <UButton v-for="m in savedMappings" :key="m.id" size="xs" variant="soft" @click="applySavedMapping(m)">
      {{ m.name }}
    </UButton>
  </div>
</div>
```

- [ ] **Step 1: Replace the step-1 chip list**

Replace the block above with:

```html
<div v-if="savedMappings.length" class="border-t border-default pt-4 space-y-2">
  <p class="text-xs text-muted">Apply a saved column mapping after upload:</p>
  <div class="flex flex-wrap gap-1.5">
    <div v-for="m in savedMappings" :key="m.id" class="group flex items-center gap-0.5">
      <template v-if="editingMappingId !== m.id">
        <UButton
          size="xs"
          variant="soft"
          :color="appliedMappingId === m.id ? 'success' : 'neutral'"
          :leading-icon="appliedMappingId === m.id ? 'i-heroicons-check' : undefined"
          @click="applySavedMapping(m)"
        >{{ appliedMappingId === m.id ? 'Applied' : m.name }}</UButton>
        <UButton
          size="xs"
          variant="ghost"
          color="neutral"
          icon="i-heroicons-pencil"
          class="opacity-0 group-hover:opacity-100 transition-opacity"
          aria-label="Rename mapping"
          @click="startRename(m)"
        />
        <UButton
          size="xs"
          variant="ghost"
          color="error"
          icon="i-heroicons-x-mark"
          class="opacity-0 group-hover:opacity-100 transition-opacity"
          aria-label="Delete mapping"
          @click="deleteMapping(m)"
        />
      </template>
      <template v-else>
        <UInput
          v-model="editingMappingName"
          size="xs"
          class="w-28"
          @keydown.enter="saveRename(m)"
          @keydown.escape="cancelRename"
        />
        <UButton
          size="xs"
          variant="ghost"
          color="success"
          icon="i-heroicons-check"
          aria-label="Save rename"
          @click="saveRename(m)"
        />
        <UButton
          size="xs"
          variant="ghost"
          color="neutral"
          icon="i-heroicons-x-mark"
          aria-label="Cancel rename"
          @click="cancelRename"
        />
      </template>
    </div>
  </div>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: update step-1 mapping chips with hover edit/delete and applied flash"
```

---

### Task 5: Update chip template — Step 2

**Files:**
- Modify: `src/components/ImportWizard.vue` (template, step 2 section)

The current step-2 chip list (around line 362–372) looks like:

```html
<div v-if="savedMappings.length" class="flex items-center gap-2 flex-wrap">
  <p class="text-xs text-muted shrink-0">Load saved mapping:</p>
  <UButton
    v-for="m in savedMappings"
    :key="m.id"
    size="xs"
    variant="soft"
    @click="applySavedMapping(m)"
  >{{ m.name }}</UButton>
</div>
```

- [ ] **Step 1: Replace the step-2 chip list**

Replace the block above with:

```html
<div v-if="savedMappings.length" class="flex items-center gap-2 flex-wrap">
  <p class="text-xs text-muted shrink-0">Load saved mapping:</p>
  <div v-for="m in savedMappings" :key="m.id" class="group flex items-center gap-0.5">
    <template v-if="editingMappingId !== m.id">
      <UButton
        size="xs"
        variant="soft"
        :color="appliedMappingId === m.id ? 'success' : 'neutral'"
        :leading-icon="appliedMappingId === m.id ? 'i-heroicons-check' : undefined"
        @click="applySavedMapping(m)"
      >{{ appliedMappingId === m.id ? 'Applied' : m.name }}</UButton>
      <UButton
        size="xs"
        variant="ghost"
        color="neutral"
        icon="i-heroicons-pencil"
        class="opacity-0 group-hover:opacity-100 transition-opacity"
        aria-label="Rename mapping"
        @click="startRename(m)"
      />
      <UButton
        size="xs"
        variant="ghost"
        color="error"
        icon="i-heroicons-x-mark"
        class="opacity-0 group-hover:opacity-100 transition-opacity"
        aria-label="Delete mapping"
        @click="deleteMapping(m)"
      />
    </template>
    <template v-else>
      <UInput
        v-model="editingMappingName"
        size="xs"
        class="w-28"
        @keydown.enter="saveRename(m)"
        @keydown.escape="cancelRename"
      />
      <UButton
        size="xs"
        variant="ghost"
        color="success"
        icon="i-heroicons-check"
        aria-label="Save rename"
        @click="saveRename(m)"
      />
      <UButton
        size="xs"
        variant="ghost"
        color="neutral"
        icon="i-heroicons-x-mark"
        aria-label="Cancel rename"
        @click="cancelRename"
      />
    </template>
  </div>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ImportWizard.vue
git commit -m "feat: update step-2 mapping chips with hover edit/delete and applied flash"
```

---

## Manual Verification Checklist

After all tasks are complete, run the app (`npm run tauri dev`) and verify:

- [ ] Mapping chips are gray (neutral) by default — no green tint
- [ ] Clicking a chip applies the mapping and briefly shows green "✓ Applied", then reverts to gray after ~1.75s
- [ ] Hovering a chip reveals pencil and × icon buttons
- [ ] Clicking pencil replaces the chip with an inline input pre-filled with the name
- [ ] Pressing Enter or clicking ✓ renames the mapping and refreshes the list
- [ ] Pressing Escape or clicking × cancels the rename without changes
- [ ] Clicking × (delete) shows a native Tauri confirmation dialog
- [ ] Confirming delete removes the mapping from the list
- [ ] Cancelling delete leaves the mapping in place
- [ ] All of the above work identically on both step 1 and step 2 of the wizard
