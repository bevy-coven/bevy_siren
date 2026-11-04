# The tour's evidence

**The still is missing, and why.** `examples/demo` is a windowed program; its picture would be a capture of the panel.
This machine could not take one when the demo was built: `screencapture` answered *could not create image from display*
(the display was asleep), and `examples/mixer`'s previously-good capture came back **black** — 1 colour in a 1280 × 720
PNG. A composed mock-up would be a lie about a project whose rule is that nothing ships without a picture it looked at, so
what is here is what this machine could prove: the crate's own numbers.

| file | what it is | how it was made |
|---|---|---|
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
