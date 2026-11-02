# One directory per iteration

`YYYY-MM-DD-NN/`, created **before** the first line of code, with `NN` the iteration's number in `../log.md` — the
newest is `-03` after `-02`, never a reused name.

```
research.md   sources with DOIs and the numbers they report; what good sounds like
plan.md       the design, the modules touched, every new field with its default and citation, the tests
eval.md       the baseline (what was already red) and the cues.yaml table, before and after
perf.md       frame cost at 8, 64 and 256 voices, before and after, with the machine's load noted
review.md     three passes — function, zoom, fresh eyes — each with a written critique, then GREAT / NOT YET / FAIL
evidence/
  before/  after/        render.txt and the WAVs from `cargo run --example render -- <dir>`
  spectrograms/          showspectrumpic and showwavespic output, including the attack and tail crops
  plots/                 a bus's gain over time, as a line an SVG script draws
  panel/                 the Cauldron audio panel or `examples/mixer` stills, small and large, with crops
```

`review.md` ends with the verdict and the earliest pass a fix would return to. A run whose verdict is not GREAT does
not ship; its `log.md` entry says what the blocker was.

## The two evidence harnesses

```sh
# Every cue, bus envelope, mix and many-voice sum, to WAVs a spectrogram and `ebur128` can read.
cargo run --release --example render -- <dir>

# The voice budget over time, with a scripted output that ends each sample: CSV, a text summary, and an SVG plot.
cargo run --release --features mixer --example budget -- <dir> [--voices 40] [--cap 32] [--life 0.05] \
    [--priority 0] [--max-instances 0]
```

`budget` needs the `mixer` feature because a voice gets a `MixerVoice` only when that backend owns playback, and the
question it answers — does an ended voice free its slot — is about that backend's retirement. Its SVG is the
artifact; `var(--c1)` and friends are the reader's theme colours, so open it where those resolve (the editor, a
browser) and rasterise with substituted colours only when you must.

