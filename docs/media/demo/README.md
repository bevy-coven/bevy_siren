# The tour's evidence

**The still is missing, and why.** `examples/demo` is a windowed program; its picture would be a capture of the panel.
This machine could not take one when the demo was built: `screencapture` answered *could not create image from display*
(the display was asleep), and `examples/mixer`'s previously-good capture came back **black** — 1 colour in a 1280 × 720
PNG. A composed mock-up would be a lie about a project whose rule is that nothing ships without a picture it looked at, so
what is here is what this machine could prove: the crate's own numbers.

| file | what it is | how it was made |
|---|---|---|
| `tour.mp4`, `tour-poster.png` | **the tour you can play**: 32 s, 1280 × 720, H.264 + AAC 192 kbit/s, 763 KB. Every frame names the scene and what it demonstrates, and the bus meters are the crate's own `SirenMix` values; the poster is frame 9.3 s (the duck at its deepest) | `tools/make_demo_video.py` |
| `bus-gain.png`, `.svg`, `.csv` | every bus's gain through the 32 s tour, from `SirenMix` — Music dipping to −14 dB under a line and recovering, Effects to −7, Ambience −9, Dialogue 0 | `cargo run --release --example demo_render -- <dir>` writes the CSV and the SVG; the PNG is the same CSV drawn at 1000 × 300 (a tool that reads pictures cannot read SVG) |
| `tour.png` | the whole tour's spectrogram, 32 s | `ffmpeg -i mix.wav -lavfi showspectrumpic=…` |
| `duck.png` | scene 4's spectrogram, 4 s | the same, on `scene-4.wav` |
| `scenes.txt` | every scene's peak, RMS, bus depths and the crate's log lines | `demo_render`'s own report |
| `live-console.txt` | the windowed demo on this Mac's device: the duck at 9.06 s, the stinger's at 17.06 s | `BEVY_ASSET_ROOT=$PWD ./target/debug/examples/demo`, 26 s |

The numbers: the tour peaks at **−3.4 dBFS** with **−21.2 dBFS RMS** and **−22.3 LUFS** integrated, LRA 8.1 LU, and no
sample at the rail. `tests/demo.rs::the_readmes_tour_numbers_are_the_runs_own` holds the README's table to `scenes.txt`.

To add the still when a display is awake:

```sh
BEVY_ASSET_ROOT=$PWD SIREN_SHOT=/tmp/demo.png SIREN_FRAMES=420 cargo run --example demo
# then move it here as demo.png and add it to docs/media/README.md and the README's hero slot
```

## `tour.mp4` — why the README links it instead of embedding a player

**GitHub's markdown renderer strips `<audio>` and `<video>`.** Measured with GitHub's own renderer
(`gh api -X POST /markdown -f mode=gfm -f context=bevy-coven/bevy_siren -f text=…`):

| input | rendered output |
|---|---|
| `<audio controls src="…/a.mp3"></audio>` | `<p dir="auto"></p>` — **gone** |
| `<video controls src="…/a.mp4"></video>` | `<p dir="auto"></p>` — **gone** |
| `![a tone](…/a.mp3)` | an `<img>` through camo pointing at the MP3, i.e. a broken image |
| `<img src="…/tour.mp4">` | an `<img>` pointing at the MP4, i.e. a broken image |
| `[▶ play](…/tour.mp4)` | a link — **works** |

The one form GitHub renders *with* a player is an attachment uploaded through its own flow
(`https://github.com/user-attachments/assets/<uuid>`). The upload endpoint (`POST /upload/policies/assets`) refuses a
token: it answered **422** to this repository's `gh` token, because that flow is the web editor's session, not the API. A
release asset is the other candidate, and the repository has **no releases** and no plan to cut one in this iteration, so
it is not a link a reader could follow today.

So the README carries the poster (a real frame), a link that plays in the browser's own player, and the per-scene table
that says what each moment demonstrates. If the tour is ever uploaded as a `user-attachments` video, its URL can replace
the link and the `<img>` of the poster in one line — the video is the same file either way.

Regenerate, from the renderer's output:

```sh
CARGO_INCREMENTAL=0 cargo run --release --example demo_render -- /tmp/demo
python3 tools/make_demo_video.py /tmp/demo docs/media/demo/tour.mp4
ffmpeg -y -ss 9.3 -i docs/media/demo/tour.mp4 -frames:v 1 docs/media/demo/tour-poster.png
```

`tests/demo.rs::the_video_is_the_tour_and_carries_its_sound` checks the file that ships: 32 s, 1280 × 720, an AAC
stream, and a poster that is a frame of it.
