# Iteration 125 — research: what makes an audio demo worth listening to

Two complaints, both right, and both measurable: **"there's a background hum"** and **"I'm not sure what we're
exemplifying."** This is what the reading says about each, and what the crate's demo therefore has to become.

## The hum

Measured on the file that shipped (`docs/loop/runs/2026-11-04-125/evidence/before/`):

| measure | value | what it means |
|---|---|---|
| the tour's quietest 50 ms frame **after the fifth second** | **−28 dBFS** | there is no silence for the last 27 s |
| frames below −50 dBFS, whole tour | **9.8 %** | and all of them are in the first five seconds |
| `music_calm`'s spectral flatness | **0.0005** | 0 is pure tones, 1 is white noise: this was three sines |
| energy below 200 Hz in that pad | **65.9 %** | the "hum" band |
| what sustained it | `wind.ogg`, a 9.6 s **noise bed**, played from scene 3 onward | a permanent broadband floor under every later scene |

So the complaint names a fact about the **source material**, not about taste: the music was an organ tone (three sines
with one slow envelope) and the ambience was a continuous noise bed. Neither ever stops, so nothing in the tour is ever
silent, and a listener hears a drone. **A demo of a mixing decision cannot have a floor**: the decision is a *change in
level*, and a floor is what hides it.

## What a demo is judged on

| source | the number or rule | how it binds this crate |
|---|---|---|
| **ITU-R BS.1770** (the loudness standard `ebur128` implements) and its practitioner summaries | match **integrated loudness** between versions being compared, **within 0.5 LU**; peak-matching is not enough because two files at one peak differ in loudness | every A/B pair in the demo is gain-matched on integrated LUFS, and the demo reports the matched values |
| **ABX methodology** (Munson & Gardner 1950; the modern summaries of it) | a difference is audible when the same choice is made **7+ times in 10**; 5:5 or 6:4 is no difference | the demo does not claim *subtle*; every pair is a change that is obvious in one listen (a machine-gun vs a walk, a line buried vs a ducked bed) |
| **game-audio reel guidance** (David Dumais; Game Audio Learning; Boechler) | **about a minute**; 1–3 examples; *"no distortion, noise or clipping"*; *"no clicks, pops, automation snaps"*; **do not put a music bed under a sound-design reel**, because it masks what is being shown; *"don't let your audio speak for itself — explain what it demonstrates"* | the demo becomes **five short vignettes**, no bed, each labelled with what to listen for and what the crate did |
| **Avižienis et al. 2004**, IEEE TDSC 1(1) | a failure is classified by whether the interface **signals** it | already applied in iteration 123; it is why vignette 4 can show a refusal |

The two rules that decide the design:

1. **One variable per vignette.** A listener cannot attribute a change if two things moved at once. Each vignette is an
   **A** (what a game without this feature does) and a **B** (the same source, through the crate), with nothing else
   changed and the two matched to within 0.5 LU.
2. **Silence between vignettes, and inside the music.** A demo with a bed under it is a demo whose differences are masked;
   that is the same reason a reel drops the music.

## What the demo therefore is

Five vignettes, each ~8 s (A, a beat of silence, B), about 45 s in total:

| # | the thing | A — without the crate | B — with the crate | what to listen for |
|---|---|---|---|---|
| 1 | **variant selection** | one recording, eight times: a machine gun | the same cue with three recordings and a no-repeat window | the foot stops sounding mechanical |
| 2 | **ducking** | a line over a bed at fixed levels: the words fight the music | the same line, `Dialogue` lowering `Music` 8 dB and `Effects` 4 | the line becomes intelligible without the bed going away |
| 3 | **the voice budget** | 40 footsteps, 40 voices, all at once | the same 40 against `max_voices: 4`: older ones are taken back | the mix does not turn to sludge, and nothing is silently dropped |
| 4 | **the layer change** | the combat stem cut in on the next frame | the same stem moved on the next **bar** of 100 bpm | the change lands on the beat instead of on a click |
| 5 | **distance** | the same sound at one level wherever it is | 4 m and 16 m through `Attenuation::Inverse` | the far one is quieter *by the law*, −12 dB at 4× |

Each is a **decision the crate makes** and a **sound a person can judge**. That is what "exemplifying" means here: not nine
features in a row, but five questions with an audible answer.

## What good sounds like

- **No floor.** Between vignettes, and between the notes of the music, the level falls to silence (measured: the phrases
  decay to −180 dBFS between notes; 31–34 % of each music file is below −60 dBFS).
- **No hum band.** No sustained low tone; the bass is plucked and decays.
- **Level matched.** A and B within 0.5 LUFS, so a listener judges the *feature* and not the gain.
- **Labelled.** Every vignette says what changed and what to listen for, in the picture and in the README.

## Predictions (written before the build; scored in `review.md`, wrong ones first)

1. **P1** — on the old tour, the quietest 50 ms frame after 5 s is **−28 dBFS** and only 9.8 % of frames are below
   −50 dBFS; on the new demo, more than 35 % of frames are below −50 dBFS (it is mostly silence and transients).
2. **P2** — the new music files have more than 30 % of frames below −60 dBFS, and their note onsets are 20 dB above the
   gap that follows them.
3. **P3** — each A/B pair matches within **0.5 LUFS** after gain, and the two are audibly different in one listen (the
   pairs are chosen to be obvious, not subtle).
4. **P4** — the video is **under 60 s** and under 3 MB, and carries an AAC stream.
5. **P5** — the README says what each vignette demonstrates, in one line each, and does not claim a subtle difference.
