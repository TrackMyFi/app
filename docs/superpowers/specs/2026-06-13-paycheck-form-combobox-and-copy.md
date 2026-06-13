# Paycheck Form: Combobox Inputs & Copy Button

**Date:** 2026-06-13  
**Status:** Approved

## Overview

Three related improvements to the paycheck workflow:
1. Replace the plain `UInput` on the Employer field with a combobox (type-to-search + free entry).
2. Apply the same combobox treatment to deduction label inputs.
3. Add a copy/duplicate button to each table row in `Paychecks.vue` that opens the Add modal pre-populated with that paycheck's values.

---

## 1. Combobox Input — `UInputMenu`

NuxtUI ships `UInputMenu`, a combobox backed by reka-ui's `ComboboxRoot`. It supports:
- Typing to filter the dropdown list
- Selecting a value from the list
- Entering a free-text value not in the list

No wrapper component is needed — `UInputMenu` works out of the box with a string array of `:items`. No custom slot templates required.

### Employer field

**Location:** `src/components/PaycheckForm.vue`

Replace:
```html
<UInput v-model="form.employer" placeholder="Employer" />
```
With:
```html
<UInputMenu v-model="form.employer" :items="knownEmployers" placeholder="Employer" />
```

`knownEmployers` is a computed property derived from `usePaychecksStore().paychecks`:
```ts
const knownEmployers = computed(() =>
  [...new Set(store.paychecks.map((p) => p.employer).filter(Boolean))]
)
```

The paychecks store is not currently imported in `PaycheckForm`. Add `usePaychecksStore` import and instantiate it.

### Deduction label fields

Each deduction row has a label `UInput`. Replace with `UInputMenu` and pass `knownDeductionLabels`:

```ts
const knownDeductionLabels = computed(() =>
  [...new Set(store.paychecks.flatMap((p) => p.deductions.map((d) => d.label)).filter(Boolean))]
)
```

```html
<UInputMenu v-model="ded.label" :items="knownDeductionLabels" placeholder="Label" />
```

---

## 2. Copy Button in the Paychecks Table

### `Paychecks.vue` changes

Add a copy icon button alongside the existing edit/delete buttons per table row:

```html
<UButton size="xs" variant="ghost" icon="i-lucide-copy" @click="openCopy(p)" />
```

Add state and handler:

```ts
const copySource = ref<Paycheck | null>(null)

function openCopy(p: Paycheck) {
  editing.value = null
  copySource.value = p
  isModalOpen.value = true
}
```

Update `openAdd` to clear `copySource`:
```ts
function openAdd() { editing.value = null; copySource.value = null; isModalOpen.value = true }
```

Update `openEdit` to clear `copySource`:
```ts
function openEdit(p: Paycheck) { editing.value = p; copySource.value = null; isModalOpen.value = true }
```

Pass `copySource` to the form and update the modal title:

```html
<UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : copySource ? 'Copy paycheck' : 'Add paycheck'">
  <template #body>
    <PaycheckForm :editing="editing" :copy-from="copySource" @saved="onSaved" />
  </template>
</UModal>
```

### `PaycheckForm.vue` changes

Add prop:
```ts
const props = defineProps<{ editing: Paycheck | null; copyFrom: Paycheck | null }>()
```

Update the `watch` on `props.editing` to also watch `props.copyFrom`. Add a third branch in the watcher:

```ts
watch(
  () => [props.editing, props.copyFrom],
  ([e, c]) => {
    saveError.value = null
    if (e) {
      // existing edit population ...
    } else if (c) {
      form.payDate = today          // override to today for the new paycheck
      form.employer = c.employer
      form.payPeriod = c.payPeriod
      form.grossAmount = c.grossAmount
      form.netAmount = c.netAmount
      form.federalTax = c.federalTax
      form.stateTax = c.stateTax
      form.localTax = c.localTax
      form.socialSecurityTax = c.socialSecurityTax
      form.medicareTax = c.medicareTax
      form.deductions = c.deductions.map((d) => ({ ...d, contributionAccountType: d.contributionAccountType ?? null, accountId: d.accountId ?? null }))
      form.employerMatch = c.employerMatch.map((m) => ({ ...m, accountId: m.accountId ?? null }))
    } else {
      resetForm()
    }
  },
  { immediate: true },
)
```

Because `editing` is `null` when copying, the form's `save()` function already routes to `store.create` — no changes needed there.

---

## Files Changed

| File | Change |
|------|--------|
| `src/components/PaycheckForm.vue` | Add `copyFrom` prop; extend watcher; add `knownEmployers` + `knownDeductionLabels` computeds; swap `UInput` → `UInputMenu` for employer and deduction labels; import paychecks store |
| `src/pages/Paychecks.vue` | Add `copySource` ref; add `openCopy`; update `openAdd`/`openEdit` to clear `copySource`; add copy button per row; pass `:copy-from` to `PaycheckForm`; update modal title |

---

## Out of Scope

- Saved/named templates (not requested)
- "Load from last paycheck for employer" shortcut (not selected)
- Combobox on employer match labels (not mentioned)
