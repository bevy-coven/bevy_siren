# plan.md — iteration 20: one answer to "is this a bus", and one sentence per fault

## The pick, and why it beats the runner-up

**Pick:** `SirenSettings::problems()` **invents four faults** for a table the crate mixes correctly. A table with
`buses: []` — documented to mean the crate's four — is reported as having four missing buses, because `problems()` builds
its own bus set instead of calling `bus_names()`, the one implementation every other reader uses. A report that cries
wolf is worse than no report: it teaches an author to ignore the panel.

Beats the runner-up (a `Refusal::NotALoop` variant): that one is a message nobody reads yet; this one makes the message
everybody reads **wrong**.

## The design

One function, in `src/mix.rs`:

1. **`problems()` reads `bus_names()`.** The empty-list rule then has one implementation, and the report and the mix agree
   by construction. (Iteration 10 fixed the identical bypass in `update_mix`; this is the last caller doing it.)
2. **A bus is named once.** A `BTreeSet<&str>` of buses already reported, shared by the `source` and `target` checks, so
   two rules naming the same missing bus — the ordinary case, since `default_ducks()` shares `Dialogue` — produce one
   sentence. The bus-name-duplication loop above already uses this shape.
3. **Nothing else changes.** The sentences, the order for genuinely distinct faults, and the `by_db`/`hold`/`fade` checks
   are untouched.

## Fields

None. No `CueTable`, `SirenSettings` or `MusicScore` field. No public type changes: `problems()` returns `Vec<String>`
and still does.

## Tests

- `an_empty_bus_list_reports_no_bus_problems` — `buses: []`: no sentence mentions a bus. **Fails on the base commit**
  (four sentences).
- `the_default_ducks_name_their_missing_source_once` — two buses: exactly one sentence for `Dialogue`. **Fails on the
  base commit** (two).
- `a_duck_naming_a_missing_bus_is_still_reported` — the "must not happen" half: a duck whose bus really is absent still
  gets its sentence, once, with the right words.
- `a_table_with_no_faults_reports_nothing` — the guard: the crate's default settings report an empty list.

## The carve

- **Not** the `BusLevel::gain_db: NaN` case: it is reported and silenced, which is a judgement the owner may want, and
  iteration 19's log records it as no item.
- **Not** any change to `bus_names()` itself: it is right, and it is the reference this fix adopts.
