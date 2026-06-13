# Paycheck Form: Combobox Inputs & Copy Button — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add combobox inputs for employer and deduction labels (free-text + suggestions from history), and a copy button in the paychecks table that pre-populates the add form.

**Architecture:** `PaycheckForm.vue` gains a `copyFrom` prop, two computed suggestion lists from the paychecks store, and swaps two `UInput` fields for `UInputMenu` with `mode="autocomplete"`. `Paychecks.vue` gains a `copySource` ref and a copy button per table row that passes the source paycheck to `PaycheckForm`.

**Tech Stack:** Vue 3 (Composition API), NuxtUI `UInputMenu` (`mode="autocomplete"` binds `v-model` to the raw string), Pinia stores.

---

## Files Changed

| File | Change |
|------|--------|
| `src/components/PaycheckForm.vue` | Import paychecks store; add `copyFrom` prop; add `knownEmployers` + `knownDeductionLabels` computeds; extend watcher; swap two `UInput` → `UInputMenu mode="autocomplete"` |
| `src/pages/Paychecks.vue` | Add `copySource` ref and `openCopy` function; update `openAdd`/`openEdit`; add copy button per row; pass `:copy-from` to form; update modal title |

---

## Task 1: Combobox suggestions in PaycheckForm

**Files:**
- Modify: `src/components/PaycheckForm.vue`

`UInputMenu` with `mode="autocomplete"` binds `v-model` directly to a string — the search term and the model value are the same thing, so it works as a plain `UInput` replacement with a suggestion dropdown on top.

- [ ] **Step 1: Import the paychecks store and add suggestion computeds**

In `src/components/PaycheckForm.vue`, after the existing imports add:

```ts
import { usePaychecksStore } from '../stores/paychecks'
```

After the `const accountsStore = useAccountsStore()` line, add:

```ts
const store = usePaychecksStore()

const knownEmployers = computed(() =>
  [...new Set(store.paychecks.map((p) => p.employer).filter(Boolean))]
)

const knownDeductionLabels = computed(() =>
  [...new Set(store.paychecks.flatMap((p) => p.deductions.map((d) => d.label)).filter(Boolean))]
)
```

- [ ] **Step 2: Swap the employer UInput for UInputMenu**

In the template, find:

```html
<UInput v-model="form.employer" placeholder="Employer" />
```

Replace with:

```html
<UInputMenu v-model="form.employer" mode="autocomplete" :items="knownEmployers" placeholder="Employer" />
```

- [ ] **Step 3: Swap the deduction label UInput for UInputMenu**

In the template, find:

```html
<UInput v-model="ded.label" placeholder="Label" />
```

Replace with:

```html
<UInputMenu v-model="ded.label" mode="autocomplete" :items="knownDeductionLabels" placeholder="Label" />
```

- [ ] **Step 4: Commit**

```bash
git add src/components/PaycheckForm.vue
git commit -m "feat: combobox inputs for employer and deduction labels"
```

---

## Task 2: copyFrom prop and watcher branch in PaycheckForm

**Files:**
- Modify: `src/components/PaycheckForm.vue`

- [ ] **Step 1: Add the copyFrom prop**

Find:

```ts
const props = defineProps<{ editing: Paycheck | null }>()
```

Replace with:

```ts
const props = defineProps<{ editing: Paycheck | null; copyFrom: Paycheck | null }>()
```

- [ ] **Step 2: Extend the watcher to handle copyFrom**

Find the entire `watch` block (lines 67–99 in the original file):

```ts
watch(
  () => props.editing,
  (e) => {
    saveError.value = null
    if (e) {
      form.payDate = e.payDate
      form.employer = e.employer
      form.payPeriod = e.payPeriod
      form.grossAmount = e.grossAmount
      form.netAmount = e.netAmount
      form.federalTax = e.federalTax
      form.stateTax = e.stateTax
      form.localTax = e.localTax
      form.socialSecurityTax = e.socialSecurityTax
      form.medicareTax = e.medicareTax
      form.deductions = e.deductions.map((d) => ({
        label: d.label,
        amount: d.amount,
        preTax: d.preTax,
        contributionAccountType: d.contributionAccountType ?? null,
        accountId: d.accountId ?? null,
      }))
      form.employerMatch = e.employerMatch.map((m) => ({
        label: m.label,
        amount: m.amount,
        accountId: m.accountId ?? null,
      }))
    } else {
      resetForm()
    }
  },
  { immediate: true },
)
```

Replace with:

```ts
watch(
  () => [props.editing, props.copyFrom] as const,
  ([e, c]) => {
    saveError.value = null
    if (e) {
      form.payDate = e.payDate
      form.employer = e.employer
      form.payPeriod = e.payPeriod
      form.grossAmount = e.grossAmount
      form.netAmount = e.netAmount
      form.federalTax = e.federalTax
      form.stateTax = e.stateTax
      form.localTax = e.localTax
      form.socialSecurityTax = e.socialSecurityTax
      form.medicareTax = e.medicareTax
      form.deductions = e.deductions.map((d) => ({
        label: d.label,
        amount: d.amount,
        preTax: d.preTax,
        contributionAccountType: d.contributionAccountType ?? null,
        accountId: d.accountId ?? null,
      }))
      form.employerMatch = e.employerMatch.map((m) => ({
        label: m.label,
        amount: m.amount,
        accountId: m.accountId ?? null,
      }))
    } else if (c) {
      form.payDate = today
      form.employer = c.employer
      form.payPeriod = c.payPeriod
      form.grossAmount = c.grossAmount
      form.netAmount = c.netAmount
      form.federalTax = c.federalTax
      form.stateTax = c.stateTax
      form.localTax = c.localTax
      form.socialSecurityTax = c.socialSecurityTax
      form.medicareTax = c.medicareTax
      form.deductions = c.deductions.map((d) => ({
        label: d.label,
        amount: d.amount,
        preTax: d.preTax,
        contributionAccountType: d.contributionAccountType ?? null,
        accountId: d.accountId ?? null,
      }))
      form.employerMatch = c.employerMatch.map((m) => ({
        label: m.label,
        amount: m.amount,
        accountId: m.accountId ?? null,
      }))
    } else {
      resetForm()
    }
  },
  { immediate: true },
)
```

- [ ] **Step 3: Commit**

```bash
git add src/components/PaycheckForm.vue
git commit -m "feat: add copyFrom prop to PaycheckForm for pre-populated create"
```

---

## Task 3: Copy button in Paychecks.vue

**Files:**
- Modify: `src/pages/Paychecks.vue`

- [ ] **Step 1: Add copySource ref and openCopy function**

Find:

```ts
const editing = ref<Paycheck | null>(null)
```

Replace with:

```ts
const editing = ref<Paycheck | null>(null)
const copySource = ref<Paycheck | null>(null)
```

Find:

```ts
function openAdd() { editing.value = null; isModalOpen.value = true }
function openEdit(p: Paycheck) { editing.value = p; isModalOpen.value = true }
```

Replace with:

```ts
function openAdd() { editing.value = null; copySource.value = null; isModalOpen.value = true }
function openEdit(p: Paycheck) { editing.value = p; copySource.value = null; isModalOpen.value = true }
function openCopy(p: Paycheck) { editing.value = null; copySource.value = p; isModalOpen.value = true }
```

- [ ] **Step 2: Add copy button to each table row**

Find the row action buttons:

```html
<td class="text-right">
  <UButton size="xs" variant="ghost" icon="i-lucide-pencil" @click="openEdit(p)" />
  <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash-2" @click="removeRow(p)" />
</td>
```

Replace with:

```html
<td class="text-right">
  <UButton size="xs" variant="ghost" icon="i-lucide-copy" @click="openCopy(p)" />
  <UButton size="xs" variant="ghost" icon="i-lucide-pencil" @click="openEdit(p)" />
  <UButton size="xs" variant="ghost" color="error" icon="i-lucide-trash-2" @click="removeRow(p)" />
</td>
```

- [ ] **Step 3: Pass copyFrom to PaycheckForm and update modal title**

Find:

```html
<UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : 'Add paycheck'">
  <template #body>
    <PaycheckForm :editing="editing" @saved="onSaved" />
  </template>
</UModal>
```

Replace with:

```html
<UModal v-model:open="isModalOpen" :title="editing ? 'Edit paycheck' : copySource ? 'Copy paycheck' : 'Add paycheck'">
  <template #body>
    <PaycheckForm :editing="editing" :copy-from="copySource" @saved="onSaved" />
  </template>
</UModal>
```

- [ ] **Step 4: Commit**

```bash
git add src/pages/Paychecks.vue
git commit -m "feat: add copy button to paychecks table row"
```

---

## Task 4: Manual verification

- [ ] **Step 1: Run the app**

```bash
npm run dev
```

Navigate to the Paychecks page.

- [ ] **Step 2: Verify employer combobox**

Click "Add paycheck". Click the Employer field — if paychecks exist, a dropdown should appear with known employer names. Type a partial name to filter. Type a brand-new name to confirm free entry works.

- [ ] **Step 3: Verify deduction label combobox**

In the form, click "Add" under Deductions. Click the label field — if prior paychecks have deductions, known labels should appear in the dropdown. Type to filter. Type a new label to confirm free entry works.

- [ ] **Step 4: Verify copy button**

In the table, click the copy icon on any row. Modal should open titled "Copy paycheck". All fields should be populated from the source paycheck, but Pay date should be today's date. Click "Add paycheck" to confirm it saves as a new record (original unchanged, new entry appears in table).

- [ ] **Step 5: Verify edit is unaffected**

Click the pencil icon on a row. Modal should open titled "Edit paycheck" with that paycheck's pay date (not today). Save and confirm no duplicate was created.
