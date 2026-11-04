# The tour's evidence

**`tour.mp4` is three A/B pairs, 26 seconds.** Each pair changes one thing and its halves are matched to within 0.5 LU on
integrated loudness (ITU-R BS.1770), so what a listener judges is the feature and not a level difference.

| file | what it is |
|---|---|
| `tour.mp4` | the tour: 1280 × 720, H.264 + AAC, 26 s. A frame names **A** or **B**, the vignette, what to listen for, each half's waveform and its measured level |
| `tour-poster.png` | frame 2 s: the `A` half of vignette 1 |
| `tour.txt` | `examples/demo_render`'s own report — every half's peak, RMS, integrated loudness, **the gain applied to match it**, and the crate's log lines |

## What is in it, and what was cut

| # | vignette | A | B | correlated? |
|---|---|---|---|---|
| 1 | variant selection | one recording, ten times at a walk's cadence | three recordings, a no-repeat window | **−0.08** |
| 2 | the voice budget | forty footsteps, forty voices | the same forty against a cap of four | **+0.87** |
| 3 | distance | the same level at 4 m and 16 m | `Attenuation::Inverse`: 12 dB down at 4× | **+0.86** |

**Correlation is the check that a pair is a pair**: 1.0 means A and B are the same sound and the vignette demonstrates
nothing.

**Two vignettes were cut, and the reason is a finding.** *Ducking* and *the layer change on the bar* both need a bed that
keeps playing while something happens over it. A sustained bed on this crate is either

- a **`Music` stem**, which `adopt_layers` hands to the score — the score then drives its level, so a vignette cannot set
  it (measured: the layer vignette's A half came out at **−180 dBFS**, silent), or
- a cue on another bus, which the table's budget **refused** in the harness (`refused \`music.calm\`: no voice free`, with
  the score's two stems already live).

The duck **rule** was measured working: `dialogue.greeting ducked Ambience by -8.0 dB` and `Effects by -4.0 dB` are in
`tour.txt` from the run where the bed was refused. But with nothing under it to duck, the two halves correlated at
**0.997** — a pair that demonstrates nothing. They are the next iteration's item rather than an A/B here that is not one.

## Why the first demo was replaced

The first version put nine features in one continuous stream with a music pad and a noise bed under them. Measured before
it was replaced (`docs/loop/runs/2026-11-04-125/evidence/before/`):

| measure | value |
|---|---|
| quietest 50 ms frame after the fifth second | **−28 dBFS** — no silence for 27 of its 32 s |
| frames below −50 dBFS | 9.8 %, all in the first five seconds |
| `music_calm` spectral flatness | **0.0005** — three pure sines (196, 294, 392 Hz) |
| energy below 200 Hz in that pad | 65.9 % |

It was heard as a hum, and nothing in it said what was being demonstrated. The sound set now uses **plucked phrases that
decay to silence between notes** (31–34 % of each music file is below −60 dBFS) and the demo is A/B pairs.

## Regenerating

```sh
python3 tools/make_demo_sounds.py assets/audio/demo           # the fourteen recordings
CARGO_INCREMENTAL=0 cargo run --release --example demo_render -- /tmp/demo
python3 tools/make_demo_video.py /tmp/demo docs/media/demo/tour.mp4
ffmpeg -y -ss 2 -i docs/media/demo/tour.mp4 -frames:v 1 docs/media/demo/tour-poster.png
ffmpeg -nostats -i docs/media/demo/tour.mp4 -af ebur128=peak=true -f null - 2>&1 | grep -E "^    (I|Peak):"
```

## Why the README links a video instead of embedding a player

**GitHub's markdown renderer strips `<audio>` and `<video>`.** Measured with GitHub's own renderer:

| input | rendered output |
|---|---|
| `<audio controls src="…/a.mp3"></audio>` | `<p dir="auto"></p>` — **gone** |
| `<video controls src="…/a.mp4"></video>` | `<p dir="auto"></p>` — **gone** |
| `![a tone](…/a.mp3)` | an `<img>` through camo pointing at the MP3, i.e. a broken image |
| `[▶ play](…/tour.mp4)` | a link — **works** |

And the MIME type matters: `raw.githubusercontent.com` serves an `.mp4` as `application/octet-stream` (a download), while
`cdn.jsdelivr.net/gh/bevy-coven/bevy_siren@main/…/tour.mp4` answers **`video/mp4`** (measured: 200, byte-identical to the
committed file), so a browser plays it in place. The one form GitHub renders *with* a player is a `user-attachments`
upload, whose endpoint refuses a token (422 to this repository's token); if the tour is ever uploaded there, its URL
replaces the link and the poster's `<img>` in one line.

Tests: `tests/demo.rs` — every pair differs and records its level match, no half reaches the rail, the video is 720p with
an AAC stream and under 2 MB, and the recordings are the lengths the generator makes.
