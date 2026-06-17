# Import Mapping Chip Management

**Date:** 2026-06-17  
**Status:** Approved

## Overview

Add rename and delete actions to saved column mapping chips in the Import Wizard, and change their appearance from green (misleadingly "active"-looking) to gray by default. When a mapping is applied, the chip briefly flashes green with "✓ Applied" feedback before reverting.

## Scope

Changes touch:
- `src-tauri/src/commands/import_mappings.rs` — new `update_import_mapping` Rust function + Tauri command
- `src-tauri/src/lib.rs` — register new command
- `src/lib/api/importMappings.ts` — new `updateImportMapping` TS export
- `src/components/ImportWizard.vue` — chip template + new refs + new handlers

## Chip Appearance

Default state: `color="neutral"` on `UButton` (gray soft chip). Applied to both the step-1 and step-2 chip lists.

Applied state: when a mapping is applied, temporarily switch that chip to `color="success"` with label `✓ Applied`. Revert after 1.75 seconds.

## Applied Animation

```
appliedMappingId: ref<number | null>(null)
appliedTimer: ref<ReturnType<typeof setTimeout> | null>(null)
```

In `applySavedMapping(m)`:
1. Apply the config (existing logic)
2. Clear any running timer
3. Set `appliedMappingId = m.id`
4. Schedule reset after 1750ms: `appliedMappingId = null`

Chip renders:
- If `appliedMappingId === m.id`: `color="success"`, label = `✓ Applied`
- Otherwise: `color="neutral"`, label = `m.name`

## Chip Hover Actions (Option A)

Each chip slot becomes a `div` with Tailwind `group` class. Pencil and × icon buttons have `opacity-0 group-hover:opacity-100 transition-opacity` so they only appear on hover.

Layout per chip:
```
[  Name (click to apply)  ] [✏ pencil] [× delete]
```

When in rename mode for a chip, the entire slot swaps to:
```
[ input (pre-filled name) ] [✓ save] [× cancel]
```

Hover actions are hidden while any chip is in rename mode (to reduce visual noise).

## Rename Flow

State:
```
editingMappingId: ref<number | null>(null)
editingMappingName: ref<string>('')
```

`startRename(m)`:
- Set `editingMappingId = m.id`, `editingMappingName = m.name`

`saveRename(m)`:
- Guard: if `editingMappingName.trim()` is empty, cancel instead
- Call `updateImportMapping(m.id, editingMappingName.trim())`
- Refresh list: `savedMappings.value = await mappingApi.listImportMappings()`
- Clear editing state

`cancelRename()`:
- Clear editing state

Keyboard: `@keydown.enter` on the input calls `saveRename`, `@keydown.escape` calls `cancelRename`.

## Delete Flow

`deleteMapping(m)`:
- Call `confirm()` from `@tauri-apps/plugin-dialog` with message `Delete "${m.name}"?`
- If confirmed: call `mappingApi.deleteImportMapping(m.id)`, refresh list

## Backend: Rust

New function in `src-tauri/src/commands/import_mappings.rs`:

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

Register in `lib.rs` alongside the existing import mapping commands.

## Backend: TypeScript

New export in `src/lib/api/importMappings.ts`:

```ts
export const updateImportMapping = (id: number, name: string) =>
  invoke<void>('update_import_mapping_cmd', { id, name })
```

## Error Handling

- If `saveRename` fails (Tauri error), show a toast via `useToast` with `color: 'error'`
- If `deleteMapping` fails, show a toast with `color: 'error'`
- Applied animation runs purely client-side; no error path

## Out of Scope

- Updating a mapping's stored config (user explicitly chose rename-only)
- Drag-to-reorder chips
- Bulk delete
