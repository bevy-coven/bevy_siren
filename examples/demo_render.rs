//! **The tour, as audio you can measure.** The windowed `examples/demo` is what a person listens to; this is what a
//! reviewer — and a test — can read, because a claim about sound needs samples.
//!
//! ```sh
//! CARGO_INCREMENTAL=0 cargo run --release --example demo_render -- <out-dir>
//! ```
//!
//! It runs the **demo's own** cue table, mix table, score and recordings (`examples/demo.rs`, included as a module), on a
//! headless app with the crate's plugin and a **recording output**: an `AudioOutput` that decodes each sample the crate
//! asks it to play, takes the bus and voice gains the crate writes, and mixes them into one buffer at the crate's own
//! numbers. Then it writes:
//!
//! - `mix.wav` — nine scenes back to back, 34 s, the sum of what the crate decided;
//! - `scene-<n>.wav` — each scene on its own, so a defect can be named by scene;
//! - `scenes.txt` — per scene: peak, RMS, the bus gains the crate wrote, the duck depths, and every log line.
//!
//! **What this is and is not.** It is the crate's *decisions* — which cue played, at what gain, on which bus, with which
//! duck, under which budget — rendered through the same `model` functions the systems use. It is not the crate's own
//! mixer: that needs a device, and a machine that can record a demo should not need one to prove it. The crate's mixer is
//! covered by its own tests (`src/mixer/mod.rs`, `tests/mixer.rs`) and by the live run this file's `scenes.txt` is
//! compared against.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use bevy::asset::AssetApp;
use bevy::audio::{AudioSource, Decodable, Source};
use bevy_siren::model::{active_duck_db, chain_db, db_to_gain, semitones_to_ratio, sum_db};
use bevy_siren::output::{AudioOutput, Backend, Output};
use bevy_siren::{CueTable, MusicState, Play, SirenChoices, SirenLog, SirenMix, SirenSettings, SirenSounds};

// **The windowed example as a module here**: its `main`, its window and its capture are never called from this harness —
// the parts used are the tables, the score and the keys. `allow(dead_code)` is on this module only.
#[allow(dead_code)]
#[path = "demo.rs"]
mod demo;

/// The demo's recordings, under `assets/` — the same constant the windowed example loads from.
const DEMO_AUDIO: &str = "audio/demo";

// The render harness's arithmetic, of which this one uses `RATE` and `at_speed`; the rest is for `examples/render`.
#[allow(dead_code)]
#[path = "render/shapes.rs"]
mod shapes;
use shapes::{RATE, at_speed};

/// **A voice as the recorder holds it**: the decoded samples, the speed the crate asked for, and the two gains it wrote.
struct Voice {
    bus: String,
    samples: Vec<f32>,
    started_at_s: f64,
    /// What the crate told the bus's gain to be, per frame; indexed by the buffer position of that frame.
    bus_db: f32,
    voice_db: f32,
    looping: bool,
    stopped_at_s: Option<f64>,
}

/// **The crate's output, recorded.** Every call the crate makes to its backend is kept: the bus gains per frame, each
/// voice's own gain, and the bytes it asked to play, decoded with the same call Bevy's path makes.
///
/// Held behind an `Arc<Mutex<…>>` because `Output` takes a `Box<dyn AudioOutput>` and a `Box` cannot be read back as the
/// concrete type: the harness keeps the handle, the world keeps the boxed proxy, and both see the same recording.
type Shared = std::sync::Arc<std::sync::Mutex<Recorder>>;

/// **The boxed proxy `Output` holds**: a newtype round the handle, because the orphan rule forbids implementing this
/// crate's trait for `Arc`, and `dyn AudioOutput` cannot be downcast back to the concrete recorder.
struct Proxy(Shared);

impl Proxy {
    fn lock(&self) -> std::sync::MutexGuard<'_, Recorder> {
        self.0.lock().expect("the recorder")
    }
}

#[derive(Default)]
struct Recorder {
    /// The bus's dB, as `update_mix` last wrote it — the crate's own decision, not the recorder's.
    bus_db: BTreeMap<String, f32>,
    voices: Vec<Voice>,
    /// The crate's clock, in seconds, so a voice's start is a position in the file.
    now_s: f64,
    /// The bytes the crate handed to `play`, and what was decoded from them.
    decoded: BTreeMap<usize, Vec<f32>>,
}

/// A stable key for the bytes the crate hands an output: the address the `Arc` points at, which is one per loaded asset.
fn key(bytes: &std::sync::Arc<[u8]>) -> usize {
    std::sync::Arc::as_ptr(bytes) as *const u8 as usize
}

/// Decode `bytes` the way the crate's Bevy path does — the same builder, the same unwrap-free error handling.
fn decode(bytes: &std::sync::Arc<[u8]>) -> Option<Vec<f32>> {
    let source = AudioSource { bytes: std::sync::Arc::clone(bytes) };
    let channels = source.bytes.len();
    let mut decoder = <AudioSource as Decodable>::Decoder::builder()
        .with_byte_len(channels as u64)
        .with_data(std::io::Cursor::new(source.clone()))
        .build()
        .ok()?;
    let per_channel = usize::from(decoder.channels().get());
    let interleaved: Vec<f32> = decoder.by_ref().collect();
    // Mono: the average of the channels. The demo's files are one signal written to both sides, so this is exact.
    Some(interleaved.chunks(per_channel).map(|f| f.iter().sum::<f32>() / per_channel as f32).collect())
}

impl AudioOutput for Proxy {
    fn set_bus_db(&mut self, bus: &str, db: f32) {
        self.lock().bus_db.insert(bus.to_owned(), db);
    }

    fn set_voice_db(&mut self, _: &str, voice: usize, db: f32) {
        if let Some(v) = self.lock().voices.get_mut(voice) {
            v.voice_db = db;
        }
    }

    fn stop_voice(&mut self, _: &str, voice: usize) {
        let mut recorder = self.lock();
        let now = recorder.now_s;
        if let Some(v) = recorder.voices.get_mut(voice) {
            v.stopped_at_s = Some(now);
        }
    }

    fn play(
        &mut self,
        bus: &str,
        bytes: &std::sync::Arc<[u8]>,
        db: f32,
        speed: f32,
        looping: bool,
    ) -> Option<usize> {
        let mut recorder = self.lock();
        let samples = match recorder.decoded.get(&key(bytes)) {
            Some(samples) => samples.clone(),
            None => {
                let samples = decode(bytes)?;
                recorder.decoded.insert(key(bytes), samples.clone());
                samples
            }
        };
        let samples = at_speed(&samples, speed);
        let bus_db = *recorder.bus_db.get(bus).unwrap_or(&0.0);
        let started_at_s = recorder.now_s;
        recorder.voices.push(Voice {
            bus: bus.to_owned(),
            samples,
            started_at_s,
            bus_db,
            voice_db: db,
            looping,
            stopped_at_s: None,
        });
        Some(recorder.voices.len() - 1)
    }

    fn finished(&self, _: &str, voice: usize) -> bool {
        let recorder = self.lock();
        let Some(v) = recorder.voices.get(voice) else { return true };
        if v.stopped_at_s.is_some() {
            return true;
        }
        let elapsed = recorder.now_s - v.started_at_s;
        let length_s = v.samples.len() as f64 / f64::from(RATE);
        if v.looping { false } else { elapsed >= length_s }
    }
}

/// **The recording, mixed**: for each sample, every live voice at its own gain and its bus's, at the crate's numbers.
///
/// A voice is audible from its start until it stops or its sample runs out; a bus's gain is the value the crate wrote for
/// the frame the sample falls in, held until the next write — the same "one number per bus per frame" the mixer's own
/// `BusGain` implements.
fn mix(recorder: &Recorder, frames: usize, bus_history: &BTreeMap<String, Vec<(f64, f32)>>) -> Vec<f32> {
    let mut out = vec![0.0f32; frames];
    for voice in &recorder.voices {
        let start = (voice.started_at_s * f64::from(RATE)) as usize;
        if start >= frames {
            continue;
        }
        let length = voice.samples.len();
        for (i, sample) in out.iter_mut().enumerate().skip(start) {
            let elapsed = i - start;
            let at = if voice.looping {
                elapsed % length
            } else {
                if elapsed >= length {
                    break;
                }
                elapsed
            };
            let t = i as f64 / f64::from(RATE);
            let bus_db = bus_history
                .get(&voice.bus)
                .and_then(|h| h.iter().rev().find(|(when, _)| *when <= t).map(|(_, db)| *db))
                .unwrap_or(voice.bus_db);
            let gain = db_to_gain(chain_db(&[bus_db, voice.voice_db]));
            *sample += voice.samples[at] * gain;
        }
    }
    out
}

fn wav(samples: &[f32], path: &PathBuf) -> std::io::Result<()> {
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        out.extend_from_slice(&((clamped * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    std::fs::write(path, out)
}

/// Peak and RMS in dB, and the count of samples at the rail — the numbers a listener's ears would report as "loud",
/// "quiet" and "clipping".
fn stats(samples: &[f32]) -> (f32, f32, usize) {
    if samples.is_empty() {
        return (f32::NEG_INFINITY, f32::NEG_INFINITY, 0);
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    let rail = samples.iter().filter(|s| s.abs() >= 0.999).count();
    (20.0 * peak.max(1e-9).log10(), 20.0 * rms.max(1e-9).log10(), rail)
}

fn main() {
    let Some(out) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: cargo run --release --example demo_render -- <out-dir>");
        std::process::exit(2);
    };
    std::fs::create_dir_all(&out).expect("the output directory");

    // **The demo's own data** — the table, the mix and the score are the windowed example's, included not copied.
    let table: CueTable = demo::table();
    let settings: SirenSettings = demo::settings();

    // **The scenes, as the demo scripts them.** A `When` is when a play happens inside its scene, in seconds — the same
    // numbers `examples/demo.rs` uses: a footstep every half second in scene 1, one burst at the scene's start, six hits
    // in one frame once scene 7 is 0.4 s in, and two distances at once in scene 8.
    #[derive(Clone, Copy)]
    enum When {
        At(f64),
        Every(f64),
    }
    /// One scene: its title, its length, and the plays it makes with when each happens.
    type Scene = (&'static str, f64, Vec<(When, Play)>);
    let scenes: [Scene; 9] = [
        (
            "1 · three footsteps",
            3.0,
            // One cadence, starting at 0: an `At(0)` *and* an `Every(0.5)` are both due on the first frame, which fired
            // two footsteps 0.00 s apart in the first run of this harness.
            vec![(When::Every(0.5), Play::cue("footstep.grass"))],
        ),
        ("2 · a burst", 2.0, vec![(When::At(0.0), Play::cue("gun.burst"))]),
        (
            "3 · a bed and a pad",
            4.0,
            vec![(When::At(0.0), Play::cue("wind")), (When::At(0.0), Play::cue("music.calm"))],
        ),
        ("4 · a line ducks the music", 4.0, vec![(When::At(0.0), Play::cue("dialogue.greeting"))]),
        ("5 · the layer changes on the bar", 4.0, vec![(When::At(0.0), Play::cue("music.combat"))]),
        ("6 · a stinger", 3.0, vec![(When::At(0.0), Play::cue("music.stinger"))]),
        ("7 · the budget", 4.0, std::iter::repeat_n((When::At(0.4), Play::cue("hit")), 6).collect()),
        (
            "8 · distance",
            4.0,
            vec![
                (When::At(0.4), Play::cue("bang").at(bevy::prelude::Vec3::new(4.0, 0.0, 0.0))),
                (When::At(0.4), Play::cue("bang").at(bevy::prelude::Vec3::new(16.0, 0.0, 0.0))),
            ],
        ),
        ("9 · a mistake", 4.0, vec![(When::At(0.4), Play::cue("footstep.grss"))]),
    ];

    let mut total_s = 0.0f64;
    let mut report = String::new();
    // **One app for the whole tour, as the windowed example is one app.** A scene is a stretch of time, not a new world:
    // a bed started in scene 3 is still playing in scene 4 (which is what a duck *is*), the calm layer started in scene 3
    // is the one scene 5 fades out, and the budget scene sees the voices the scenes before it left. The first version of
    // this harness built a fresh app per scene and measured nine unrelated fragments — scene 4's duck had no music under
    // it to lower, which the spectrogram showed and no log line would have.
    let mut app = bevy::app::App::new();
    app.add_plugins((bevy::MinimalPlugins, bevy::asset::AssetPlugin::default()));
    app.init_asset::<bevy::audio::AudioSource>();
    app.init_asset_loader::<bevy::audio::AudioLoader>();
    app.world_mut().spawn((
        bevy::prelude::Transform::default(),
        bevy::prelude::GlobalTransform::default(),
        bevy::audio::SpatialListener::new(1.0),
    ));
    app.init_resource::<CueTable>();
    app.init_resource::<SirenSounds>();
    app.insert_resource(table.clone());
    app.insert_resource(settings.clone());
    app.insert_resource(demo::score());
    app.add_plugins(bevy_siren::SirenPlugin);
    app.insert_resource(Backend(true));
    let recorder: Shared = Shared::default();
    app.world_mut().insert_resource(Output(Box::new(Proxy(Shared::clone(&recorder)))));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
    {
        let table = table.clone();
        let server = app.world().resource::<bevy::asset::AssetServer>().clone();
        if let Some(mut sounds) = app.world_mut().get_resource_mut::<SirenSounds>() {
            sounds.load_table(&server, &table, DEMO_AUDIO);
        }
    }
    for _ in 0..400 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let loaded = {
            let sounds = app.world().resource::<SirenSounds>();
            let server = app.world().resource::<bevy::asset::AssetServer>();
            !sounds.is_empty()
                && sounds.keys().all(|key| {
                    sounds.get(key).and_then(|h| server.get_load_state(h.id())).is_some_and(|s| {
                        matches!(s, bevy::asset::LoadState::Loaded | bevy::asset::LoadState::Failed(_))
                    })
                })
        };
        if loaded {
            break;
        }
    }
    let mut elapsed_total = 0.0f64;
    let mut log_mark: u64 = 0;
    // **One bus history for the whole tour**, on the tour's clock: `update_mix` writes a bus's dB per frame, and a scene's
    // slice of the mix needs the gains that were in force then.
    let mut bus_history: BTreeMap<String, Vec<(f64, f32)>> = BTreeMap::new();
    for (index, (title, len_s, plays)) in scenes.iter().enumerate() {
        // The scene's own report block.
        let mut log = String::new();
        // **When each play happens**, on the scene's own clock: `At` once, `Every` on its cadence. The demo's walk resyncs
        // rather than catching up — it fires a step, then the next is due half a second from *now* — and this mirrors it,
        // so the WAV has the same rhythm the window plays.
        let frames = (*len_s * f64::from(RATE)) as usize;
        let mut fired_at: Vec<Option<f64>> = vec![None; plays.len()];
        let mut next_repeat: Vec<Option<f64>> = plays
            .iter()
            .map(|(when, _)| if matches!(when, When::Every(_)) { Some(0.0) } else { None })
            .collect();
        let scene_start_s = elapsed_total;
        let mut elapsed = 0.0f64;
        while elapsed < *len_s {
            for (i, (when, play)) in plays.iter().enumerate() {
                let due = match when {
                    When::At(at) => fired_at[i].is_none() && elapsed + 1e-9 >= *at,
                    When::Every(_) => next_repeat[i].is_some_and(|next| elapsed + 1e-9 >= next),
                };
                if !due {
                    continue;
                }
                app.world_mut().write_message(play.clone());
                match when {
                    When::At(at) => fired_at[i] = Some(*at),
                    When::Every(cadence) => {
                        // A frame longer than the cadence does not fire twice: the next step is due from now.
                        while next_repeat[i].is_some_and(|next| next <= elapsed) {
                            next_repeat[i] = Some(next_repeat[i].unwrap_or(0.0) + cadence);
                        }
                    }
                }
            }
            app.update();
            elapsed += 0.016;
            // **The output's clock is the harness's**, because a headless app has no device and `Time<Real>` moves on the
            // wall; a voice's start, its end and a bus's gain are all positions in the file, so they are written here.
            // A voice's position in the recorder's timeline is the **tour's** clock, not the scene's: the whole tour is
            // one stream of audio, and a scene is a window on it.
            recorder.lock().expect("the recorder").now_s = scene_start_s + elapsed;
            for bus in ["Music", "Effects", "Ambience", "Dialogue"] {
                let db = app.world().resource::<SirenMix>().bus_db(bus);
                bus_history.entry(bus.to_owned()).or_default().push((scene_start_s + elapsed, db));
            }
        }
        elapsed_total += *len_s;

        // **The scene's audio is a window on the tour's**, mixed from the same recorder and sliced out of the whole.
        let recorded = recorder.lock().expect("the recorder");
        let all = mix(&recorded, ((elapsed_total) * f64::from(RATE)) as usize, &bus_history);
        let from = (scene_start_s * f64::from(RATE)) as usize;
        let samples: Vec<f32> = all.iter().skip(from).take(frames).copied().collect();
        let (peak, rms, rail) = stats(&samples);
        wav(&samples, &out.join(format!("scene-{}.wav", index + 1))).expect("a scene file");
        drop(recorded);

        // **The scene's own decisions**: the entries pushed since the last scene's mark, which is what `SirenLog::since`
        // is for. A continuous app writes one log, and a scene's report is its slice of it.
        let log_lines: Vec<String> = {
            let log = app.world().resource::<SirenLog>();
            let lines: Vec<String> = log.since(log_mark).map(|e| e.line()).collect();
            log_mark = log.pushed();
            lines
        };
        log.push_str(&format!(
            "== {title}   {len_s:.1} s\n   peak {peak:.1} dBFS   rms {rms:.1} dBFS   at the rail {rail}\n"
        ));
        for (bus, history) in &bus_history {
            let deepest = history.iter().map(|(_, db)| *db).fold(f32::INFINITY, f32::min);
            log.push_str(&format!("   {bus:<10} deepest {deepest:.1} dB\n"));
        }
        for line in &log_lines {
            log.push_str(&format!("   {line}\n"));
        }
        log.push('\n');
        report.push_str(&log);
        total_s += len_s;
        println!("scene {} done, {} log lines", index + 1, log_lines.len());
    }

    // **The whole tour, one file**: the same recorder at its full length, which is what a listener hears end to end.
    let whole = {
        let recorded = recorder.lock().expect("the recorder");
        mix(&recorded, (elapsed_total * f64::from(RATE)) as usize, &bus_history)
    };
    wav(&whole, &out.join("mix.wav")).expect("the mix");
    let (peak, rms, rail) = stats(&whole);
    report.push_str(&format!(
        "== the whole tour   {total_s:.1} s   peak {peak:.1} dBFS   rms {rms:.1} dBFS   at the rail {rail}\n"
    ));
    let mut file = std::fs::File::create(out.join("scenes.txt")).expect("scenes.txt");
    file.write_all(report.as_bytes()).expect("write");

    // **A bus's gain over time, as a line.** A duck, a layer change and a recovery are *levels*; a spectrogram of the sum
    // hides them under whatever else is playing (scene 4's own RMS is dominated by the line the duck is reacting to), and
    // `showwavespic` draws audio, which turns a gain curve into a blob. So the crate's own numbers are written twice:
    // `bus-gain.csv` for a reader, and `bus-gain.svg` for the picture — one polyline per bus, with the scene boundaries
    // marked so a reader can name the moment a line moves.
    {
        let mut csv = String::from("t_s,bus,db\n");
        for (bus, history) in &bus_history {
            for (t, db) in history {
                csv.push_str(&format!("{t:.3},{bus},{db:.2}\n"));
            }
        }
        std::fs::write(out.join("bus-gain.csv"), csv).expect("bus-gain.csv");

        let (w, h, pad) = (1000.0f64, 260.0f64, 36.0f64);
        let top = 0.0f64;
        let bottom = -30.0f64;
        let x = |t: f64| pad + (t / elapsed_total) * (w - 2.0 * pad);
        let y = |db: f64| pad + ((top - db.clamp(bottom, top)) / (top - bottom)) * (h - 2.0 * pad);
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">
<g font-family=\"Menlo,monospace\" font-size=\"11\" fill=\"var(--muted)\">
<text x=\"8\" y=\"20\" fill=\"var(--fg)\">bevy_siren · the tour · each bus's gain, the crate's own numbers</text>"
        );
        for db in [0.0, -6.0, -12.0, -18.0, -24.0, -30.0] {
            svg.push_str(&format!(
                "<text x=\"4\" y=\"{:.0}\">{db:.0}</text><line x1=\"{:.0}\" y1=\"{:.0}\" x2=\"{:.0}\" y2=\"{:.0}\" stroke=\"var(--border)\" stroke-width=\"0.5\"/>",
                y(db) + 4.0,
                pad,
                y(db),
                w - pad,
                y(db)
            ));
        }
        // Scene boundaries, labelled by number: the moments the demo's script changes.
        let mut scene_start = 0.0f64;
        for (i, (_, len_s, _)) in scenes.iter().enumerate() {
            svg.push_str(&format!(
                "<line x1=\"{:.0}\" y1=\"{:.0}\" x2=\"{:.0}\" y2=\"{:.0}\" stroke=\"var(--border)\" stroke-width=\"1\"/><text x=\"{:.0}\" y=\"{:.0}\" text-anchor=\"middle\">{}</text>",
                x(scene_start),
                pad,
                x(scene_start),
                h - pad,
                x(scene_start + len_s / 2.0),
                h - pad + 14.0,
                i + 1
            ));
            scene_start += len_s;
        }
        let colours = [
            ("Music", "var(--c1)"),
            ("Effects", "var(--c2)"),
            ("Ambience", "var(--c3)"),
            ("Dialogue", "var(--c4)"),
        ];
        for (i, (bus, colour)) in colours.iter().enumerate() {
            let Some(history) = bus_history.get(*bus) else { continue };
            let points: String =
                history.iter().map(|(t, db)| format!("{:.1},{:.1} ", x(*t), y(f64::from(*db)))).collect();
            svg.push_str(&format!(
                "<polyline fill=\"none\" stroke=\"{colour}\" stroke-width=\"2\" points=\"{}\" /><text x=\"{:.0}\" y=\"{:.0}\" fill=\"{colour}\">{bus}</text>",
                points.trim_end(),
                pad + 8.0 + (i as f64) * 92.0,
                20.0
            ));
        }
        svg.push_str("</g></svg>\n");
        std::fs::write(out.join("bus-gain.svg"), svg).expect("bus-gain.svg");
    }
    // The music score is the demo's, and it is what makes the layer scenes land on the bar; print what it says so a
    // reader does not have to trust the prose.
    let score = demo::score();
    println!(
        "score: {} layers at {:.0} bpm, {:.0} beats a bar — a bar is {:.1} s; wrote {} files to {}",
        score.layers.len(),
        score.tempo_bpm,
        score.beats_per_bar,
        60.0 / f64::from(score.tempo_bpm) * f64::from(score.beats_per_bar),
        10,
        out.display()
    );
    // Named so an unused import cannot hide a decision: the crate's own duck curve is what the mix above is compared to.
    let _ = (active_duck_db, sum_db, semitones_to_ratio, MusicState::default(), SirenChoices::new(0));
}
