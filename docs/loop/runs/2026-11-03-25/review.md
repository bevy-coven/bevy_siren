# review.md — iteration 25, three passes

## Pass 1 — function

**The claim.** `examples/mixer` is the crate's only **visible** harness, the skill names it for judging a visible change,
and its module doc's first command is `cargo run --release --example mixer`. Its capture knobs are documented beside three
that default. Run it with `SIREN_FRAMES` and no `SIREN_KEYS` — the default path — and it **panics**:

```
thread 'Compute Task Pool (4)' panicked at examples/mixer.rs:376:33:
attempt to calculate the remainder with a divisor of zero
  (no still written)
```

| row | source | before | after |
|---|---|---|---|
| `SIREN_FRAMES=140`, `SIREN_KEYS` unset | S1 (the doc lists the knobs as optional), S4 (`% 0` panics in release) | **panic**, no still | `siren: frames 140`, still written |
| `SIREN_KEYS=bogus` | S2 (fail loud) | panic (empty list) | one line naming the typo, then the default script |
| `SIREN_KEYS=layer` | S3 (explicit path) | works | **unchanged** — still written |
| no environment at all | S1 | panic | 20 s, no panic |

**Against the sources.** S1 is the whole defect: the doc's framing is that these knobs *tune* a working capture, and one
of them was load-bearing. S2 is why the typo case got a line rather than a second silent fallback — the **silence** was
the bug. S4 is why this could not be dismissed as a debug-only assertion: `%` on an integer panics in every profile, and
`--release` is what the doc tells you to run.

**The severity:** every visible change this loop has shipped since iteration 1 has been evidenced by the headless
`examples/render`, because the panel harness could not produce a picture. This restores the path the skill asks for.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{mixer-1280,mixer-1920,zoom-buses,zoom-log}.png`, `docs/media/mixer-panel.gif`, and the two run
transcripts.

**The panel itself, looked at for the first time.** Four lines at 1280×720:

```
bevy_siren: 9 cues, 4 buses, 4 voices
Music     -9.3 dB   duck -8.0 dB   2 / 12 voices
Effects   -3.5 dB   duck -4.0 dB   1 / 2 voices
Ambience  -6.0 dB                 0 / 12 voices
Dialogue  +0.0 dB                 1 / 12 voices
```

Every number is one a reader can check against the log beneath it: `dialogue.greeting` played on `Dialogue`, the log
says it ducked `Music` by −8.0 and `Effects` by −4.0, and those are the two `duck` figures on the rows. `Effects` shows
`1 / 2` because its cap is two and one idle loop was stolen (`stopped engine.idle: stolen by sword.hit`). The header's
`4 voices` is the sum of the live column. **The panel explains itself without the code** — the skill's pass-3 test,
met.

**A crop mistake, mine:** `zoom-buses.png` starts at `Ambience` and looks like `Music` is missing. It is not — the crop's
y-offset was 150 px and the Music row is at 150. The full 1280 shot shows all four in `DEFAULT_BUSES` order. Recorded
rather than quietly re-cropping.

**The still is byte-stable across runs?** No, and it should not be: the scripted run's log lines carry real times
(`1.27s`, `0.89s`) because the duck fires on a frame, not a wall clock. The **decisions** are identical every run; the
timestamps are not.

**Verdict: GREAT** after the crop correction.

## Pass 3 — fresh eyes

**A different shape.** Four paths exercised: no environment (20 s, no panic), `SIREN_FRAMES` only (the default script,
still written), `SIREN_KEYS=bogus` (the notice), and `SIREN_KEYS=layer` (the explicit path, unchanged). The GIF is
**18 frames** at 6 fps from the `SIREN_SHOT_DIR` knob, which is the third knob the doc lists and the first time this loop
has used it.

**Can a developer understand the panel without the code?** Yes — that is pass 2's finding. The header counts the cues,
buses and voices the table and the world hold; the four bus rows carry level, duck and `live / cap`; the cue list carries
bus, mode, gain and priority; the log carries `played`/`stopped`/`ducked` with times. A person can see a duck recover, a
voice be stolen and a layer enter.

**What a fresh reader would still ask:** *why does the default script end with `duck`?* Because the module doc says so —
*"the last one before the still is the duck, which is still holding when the screenshot is taken"* — and the default now
is that documented script rather than nothing.

**Verdict: GREAT.**
