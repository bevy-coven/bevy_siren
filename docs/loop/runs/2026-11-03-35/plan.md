# plan.md — iteration 35: the report's doc is the whole list

## The pick, and why it beats the runner-up

**Pick:** `MusicScore::problems`'s doc lists **four** checks; the function performs **nine**. The two this loop added — a gain
that is not a number (19) and a layer on the wrong bus (34) — were never added to it, so **a reader trusts a list and
stops**. `MusicLayer::cue`'s doc has the mirror-image defect: it still says *"should"* after iteration 34 made that a
**refusal**.

Beats the runner-up (a gain above 0 dB reported as a fault): that is a **judgement call** — a boost is a legitimate mixing
move, and a score's layers crossfade rather than sum, so any threshold cries wolf. Written to
`needs-decision/above-unity-gain.md` and not built.

## The design

1. **The doc becomes the nine**, each a clause, with the note that the plugin **enforces** the last two and writes the same
   sentence to the log.
2. **`MusicLayer::cue` says `must`**, and says what happens otherwise.
3. **A test walks the nine**: one score carrying every fault, asserting each sentence; plus the empty-score case for the
   ninth. The doc and the code are now the same length.

## Fields

None. No behaviour change at all: this iteration makes the **description** match the **mechanism** that two iterations
already shipped.

## Tests

- `a_score_reports_every_fault_its_doc_lists` — every one of the eight sentences from a single score, and the ninth
  (`the score has no layers`) from an empty one. It passes on the base commit too, because the **behaviour** was already
  there; what it pins is that the list cannot silently grow or shrink without this test's needles being edited.
- The full gate, plus `RUSTDOCFLAGS="-D warnings"` — which caught a redundant intra-doc link in my first version.

## The carve

- **Not** a `CueProblem` or score sentence for a gain above 0 dB: a judgement, in `needs-decision/above-unity-gain.md`, with
  the arithmetic (a layer at +12 dB is 3.98×; `sum_db` does not clip) and why a static check on a crossfading score would
  fire on the ordinary case.
- **Not** the panel's Music block, deferred again with its budget unchanged (12 px free, ~134 needed).
