# Media

Every file here was captured from `examples/mixer` on macOS (arm64, Bevy 0.20, `bevy_siren` 0.1.0) with the commands
below. A media file a published README names is never moved; a re-capture replaces it in place.

| File | What it shows | Captured |
|---|---|---|
| `mixer.png` | The audio panel at frame 200 of a scripted run: four bus rows with their levels, the dialogue duck on the music, a bus at its cap after a steal, the cue table and the log tail. 1280 × 720. | 2026-11-02 |
| `mixer.gif` | The same run as motion, 960 px wide: the duck arriving and recovering, the steal, and the combat layer entering on the bar. | 2026-11-02 |

## Regenerating

Build once in release (the capture runs 200 frames, and the debug build's frame times are not the ones the panel's
`mean_ms` line should report):

```sh
cargo build --release --examples
```

**The still:**

```sh
SIREN_FRAMES=200 SIREN_SHOT=docs/media/mixer.png \
  SIREN_KEYS=steal,layer,duck SIREN_KEY_EVERY=20 SIREN_SIZE=1280x720 \
  cargo run --release --example mixer
```

**The frames for the GIF**, then the palette command the Coven uses for every GIF (a per-clip palette, so the flat
panel greys do not band):

```sh
rm -rf /tmp/siren-gif && mkdir -p /tmp/siren-gif
SIREN_FRAMES=260 SIREN_SHOT=/tmp/siren-gif/still.png SIREN_SHOT_DIR=/tmp/siren-gif \
  SIREN_SHOT_FROM=30 SIREN_SHOT_EVERY=2 SIREN_KEYS=steal,layer,duck SIREN_KEY_EVERY=20 SIREN_SIZE=1280x720 \
  cargo run --release --example mixer

ffmpeg -y -framerate 30 -pattern_type glob -i '/tmp/siren-gif/f_*.png' \
  -vf "fps=15,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=192[p];[b][p]paletteuse=dither=sierra2_4a" \
  docs/media/mixer.gif
```

Check the result with `read docs/media/mixer.gif` before committing it: a bus row whose dB disagrees with the panel's
own `SirenMix`, or a log pane whose lines do not match `SirenLog`, is a defect in the example and not a bad capture.
The GIF must stay under 4 MB and 960 px wide; `ffprobe docs/media/mixer.gif` prints both.
