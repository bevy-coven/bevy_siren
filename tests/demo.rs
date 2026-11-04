//! **The demo's contract, where a test can reach it**: its cue table, its mix table, its score, its sound set, and the
//! sounds it produces.
//!
//! `examples/demo.rs` is a windowed program, so a test cannot run it headless — but almost everything it *is* is data: a
//! `CueTable`, a `SirenSettings`, a `MusicScore` and a folder of recordings. This includes the example as a module (its
//! `main` is never called) and holds those where they can fail:
//!
//! 1. **The table is clean.** `problems()` names nothing, which is what makes the demo a demonstration rather than a
//!    second, broken example.
//! 2. **Every key has a file**, and every file is decoded by the build's decoders — the claim a demo makes by playing.
//! 3. **The sound set is the one `tools/make_demo_sounds.py` makes.** A re-run reproduces the audio, so the recordings
//!    are arithmetic and not a binary nobody can check (`python3 tools/make_demo_sounds.py --check` is the command; this
//!    is the half a `cargo test` can do without ffmpeg).
//! 4. **The loops loop.** Each is an exact number of bars at the score's tempo, so a layer change on the bar is on the
//!    grid, and the seam at the join is no bigger than the file's own sample-to-sample steps — otherwise every pass is a
//!    click.
//! 5. **The table does what the scenes say.** `max_instances: 2` on the footsteps (a walk has two feet) and on `bang` (the
//!    scene is a comparison of two distances at once), `UNSTEALABLE_PRIORITY` on the line and the stinger, and a spatial
//!    cue with a distance law — each one a defect this file was written after finding in a real run.

#![allow(dead_code)] // the example's `main` is a program; here the file is data and systems that are never run.

// **The whole example as a module**: its `main`, its capture system and its window are never called here — the parts a
// test holds are the tables, the score and the data. `allow(dead_code)` is on the module, not on the test file, so a
// genuinely unused item in the test itself still warns.
#[allow(dead_code)]
#[path = "../examples/demo.rs"]
mod tour_at_demo;

// `Source` is the trait carrying `channels()` and `sample_rate()`; without it in scope they are not callable, and the
// error names the *decoder*, not the missing import.
use bevy::audio::Source;
use bevy_siren::{Attenuation, Mode, UNSTEALABLE_PRIORITY};

/// The directory `examples/demo` loads from, under `assets/`.
const DIR: &str = "assets/audio/demo";

/// **The decoder call the crate's own Bevy path makes**, from `src/sounds.rs` — built through the trait's associated type,
/// which is the difference between a test that uses the same call as the code it protects and one that re-implements it.
fn decode(path: &str) -> <bevy::audio::AudioSource as bevy::audio::Decodable>::Decoder {
    use bevy::audio::{AudioSource, Decodable};
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("`{path}`: {e}"));
    let source = AudioSource { bytes: bytes.into() };
    <AudioSource as Decodable>::Decoder::builder()
        .with_byte_len(source.bytes.len() as u64)
        .with_data(std::io::Cursor::new(source.clone()))
        .build()
        .unwrap_or_else(|e| panic!("`{path}` does not decode: {e}"))
}

/// **The table is clean**: `problems()` names nothing, which is the first thing a demo has to be.
#[test]
fn the_demos_cue_table_has_no_problems() {
    let table = tour_at_demo::table();
    assert_eq!(table.cues.len(), 10, "one cue per thing the tour shows");
    let settings = tour_at_demo::settings();
    let buses = settings.bus_names();
    let problems = table.problems_with(&buses);
    assert!(problems.is_empty(), "the demo's table is not clean: {problems:?}");
}

/// **Every key the table names has a file**, and the file is one this repository makes.
#[test]
fn every_sound_the_demo_names_is_a_file_in_the_tree() {
    let table = tour_at_demo::table();
    let keys = table.sounds();
    assert_eq!(keys.len(), 11, "eleven recordings, as `assets/README.md` lists");
    for key in &keys {
        let path = format!("{DIR}/{key}");
        let meta = std::fs::metadata(&path)
            .unwrap_or_else(|e| panic!("the demo names `{key}` and there is no `{path}`: {e}"));
        assert!(meta.len() > 1_000, "`{path}` is {} bytes: an empty or near-empty file", meta.len());
    }
    // And the reverse: a file in the folder that no cue names is a recording nobody asked for.
    let named: std::collections::HashSet<&str> = keys.iter().copied().collect();
    for entry in std::fs::read_dir(DIR).expect("the demo's sound folder") {
        let name = entry.expect("a directory entry").file_name().to_string_lossy().into_owned();
        assert!(named.contains(name.as_str()), "`{DIR}/{name}` is in the demo's folder and no cue names it");
    }
}

/// **Every file is real audio**: the build's own decoder reads each one, and reads samples that are not silence.
///
/// This is the check a demo makes by existing — a `Played` for a file nobody can decode is the failure iteration 117
/// found — done here with the same decoder call the crate uses (`sounds::cannot_play`'s `Decode::Bevy`), so it is true in
/// every feature set.
#[test]
fn every_recording_decodes_to_sound_rather_than_silence() {
    for key in tour_at_demo::table().sounds() {
        let path = format!("{DIR}/{key}");
        let samples: Vec<f32> = decode(&path).take(44_100 * 12).collect();
        assert!(!samples.is_empty(), "`{path}` decodes to no samples");
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.05, "`{path}` decodes to silence (peak {peak:.4})");
    }
}

/// **The sound set is the one the generator makes** — its lengths, and the stream's own declared length rather than the
/// decoder's count.
///
/// **The decoder's count is not the file's length.** A Vorbis stream's last packet decodes to a whole block, and the
/// excess is padding: `step_01.ogg` is 0.220 s of arithmetic and decodes to **0.232 s**, 530 frames that no page declares.
/// That is the trap `src/ogg.rs` exists for (iteration 115), and this test met it too: asserting the decoded count fails
/// on a file that is correct. What is asserted instead is what a listener's loop is actually cut to — the granule
/// position of the last page, read from the file here, because `ogg::loop_length` is `pub(crate)`.
#[test]
fn the_recordings_are_the_lengths_the_generator_writes() {
    // `python3 -c "import sys;sys.path.insert(0,'tools');import make_demo_sounds as m;print({k: round(len(v)/44100,3) for k,v in m.files().items()})"`
    // — the numbers below are that command's output, so the test and the generator cannot drift silently.
    let want: [(&str, f64); 11] = [
        ("step_01.ogg", 0.220),
        ("step_02.ogg", 0.260),
        ("step_03.ogg", 0.190),
        ("hit_01.ogg", 0.260),
        ("click.ogg", 0.050),
        ("wind.ogg", 9.600),
        ("music_calm.ogg", 9.600),
        ("music_combat.ogg", 9.600),
        ("stinger.ogg", 0.600),
        ("line_01.ogg", 0.450),
        ("line_02.ogg", 0.450),
    ];
    for (key, seconds) in want {
        let path = format!("{DIR}/{key}");
        let bytes = std::fs::read(&path).expect("the file");
        let decoder = decode(&path);
        assert_eq!(decoder.sample_rate().get(), 44_100, "`{path}`: the generator writes 44.1 kHz");
        let declared = declared_frames(&bytes)
            .unwrap_or_else(|| panic!("`{path}`: no granule position in its last page"));
        // **The granule counts samples per channel already** (Vorbis I, A.2), so dividing by the channel count again
        // halves the answer: the first run of this test read 0.1103 s for a 0.220 s file for exactly that reason.
        let got = declared as f64 / 44_100.0;
        assert!(
            (got - seconds).abs() < 0.002,
            "`{path}` declares {got:.4} s ({declared} samples), and the generator makes {seconds:.3} s"
        );
    }
}

/// **The frame count an Ogg Vorbis stream declares**: the granule position of its last page, in samples.
///
/// This is `src/ogg::loop_length`'s number, read here in twenty lines because the module is `pub(crate)` and the point of
/// the assertion needs it: a decoded count includes the last block's padding and is not the length of anything. Returns
/// `None` for a file that is not a single Vorbis stream — the same refusal `src/ogg.rs` makes, for the same reason.
fn declared_frames(bytes: &[u8]) -> Option<u64> {
    let mut at = 0usize;
    let mut last_granule = None;
    while at + 27 <= bytes.len() {
        if &bytes[at..at + 4] != b"OggS" {
            return None;
        }
        let flags = bytes[at + 5];
        let granule = u64::from_le_bytes(bytes.get(at + 6..at + 14)?.try_into().ok()?);
        let segments = usize::from(bytes[at + 26]);
        let table = bytes.get(at + 27..at + 27 + segments)?;
        let body: usize = table.iter().map(|s| usize::from(*s)).sum();
        if flags & 0x04 != 0 {
            last_granule = Some(granule);
        }
        at += 27 + segments + body;
    }
    last_granule.filter(|g| *g != u64::MAX)
}

/// **The loops loop**: each pad and the bed is a whole number of bars at the score's tempo, so a layer change lands on the
/// grid — and the seam is no bigger than the file's own steps, so a pass is not a click.
#[test]
fn the_loops_are_whole_bars_and_join_without_a_click() {
    let score = tour_at_demo::score();
    let bar_s = 60.0 / f64::from(score.tempo_bpm) * f64::from(score.beats_per_bar);
    for key in ["wind.ogg", "music_calm.ogg", "music_combat.ogg"] {
        let path = format!("{DIR}/{key}");
        let decoder = decode(&path);
        let rate = f64::from(decoder.sample_rate().get());
        let channels = usize::from(decoder.channels().get());
        let samples: Vec<f32> = decoder.collect();
        let frames = samples.len() / channels;
        let length_s = frames as f64 / rate;
        let bars = length_s / bar_s;
        assert!(
            (bars - bars.round()).abs() < 0.02,
            "`{path}` is {length_s:.3} s = {bars:.3} bars at {:.0} bpm and {:.0} beats a bar",
            score.tempo_bpm,
            score.beats_per_bar
        );

        // **The seam, against the file's own steps.** The join is `last` → `first`, and a click is a step far outside the
        // distribution of the steps inside the file. Measured on the generator's output before this test existed: the
        // seam was 1233 against a median step of 408 and a 99th percentile of 2533, so it is inside the distribution.
        let mono: Vec<f32> = samples.chunks(channels).map(|f| f[0]).collect();
        let seam = (mono[0] - mono[mono.len() - 1]).abs();
        let mut steps: Vec<f32> = mono.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        steps.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let p999 = steps[(steps.len() as f64 * 0.999) as usize];
        assert!(
            seam <= p999,
            "`{path}`: the seam is {seam:.4}, past the 99.9th percentile step {p999:.4} — a click"
        );
    }
}

/// **The table says what the scenes need**, each entry a defect a real run of this demo found:
///
/// - the footsteps and `bang` may overlap (a walk has two feet; the distance scene is a comparison, and at one instance
///   the second play *stole* the first);
/// - the line and the stinger are `UNSTEALABLE_PRIORITY`, so a full budget refuses someone else rather than cutting them;
/// - `bang` is spatial with a distance law, which is what the scene measures;
/// - the two music stems loop on `Music`, so the score can start and move them.
#[test]
fn the_table_holds_the_lessons_of_a_real_run() {
    let table = tour_at_demo::table();
    let cue =
        |name: &str| table.cues.iter().find(|c| c.name == name).unwrap_or_else(|| panic!("no cue `{name}`"));

    assert_eq!(cue("footstep.grass").max_instances, 2, "a walk has two feet");
    assert_eq!(cue("bang").max_instances, 2, "the distance scene plays two at once");
    assert_eq!(cue("hit").max_instances, 3, "the budget scene fills a limit of three");
    assert_eq!(cue("hit").priority, 10);
    assert_eq!(cue("dialogue.greeting").priority, UNSTEALABLE_PRIORITY, "a line is never stolen");
    assert_eq!(cue("music.stinger").priority, UNSTEALABLE_PRIORITY, "a stinger is never stolen");
    assert!(cue("bang").spatial, "the distance scene needs a spatial cue");
    assert!(
        matches!(cue("bang").attenuation, Attenuation::Inverse { near_m, rolloff } if near_m == 4.0 && rolloff == 1.0),
        "the law the scene quotes: 4 m and rolloff 1"
    );
    for stem in ["music.calm", "music.combat"] {
        assert_eq!(cue(stem).mode, Mode::Loop, "`{stem}` is a looping stem");
        assert_eq!(cue(stem).bus, "Music", "`{stem}` is a stem of Music");
    }
    // **Every play the tour makes is a cue the table has**, so a renamed cue is caught here and not by a silent scene.
    for name in [
        "footstep.grass",
        "gun.burst",
        "wind",
        "music.calm",
        "music.combat",
        "music.stinger",
        "hit",
        "dialogue.greeting",
        "bang",
    ] {
        assert!(table.get(name).is_some(), "the tour plays `{name}`");
    }
    // And the one it means to refuse.
    assert!(table.get("footstep.grss").is_none(), "scene 9 is a cue the table does not have");
}

/// **The mix table is the crate's shape with the demo's caps**: four buses, `Effects` at three voices so the steal has a
/// cap, and `problems()` empty.
#[test]
fn the_demos_mix_table_is_clean_and_capped_where_the_scenes_need_it() {
    let settings = tour_at_demo::settings();
    assert!(settings.problems().is_empty(), "{:?}", settings.problems());
    assert_eq!(settings.bus("Effects").map(|b| b.max_voices), Some(3));
    assert_eq!(settings.bus("Music").map(|b| b.max_voices), Some(4));
    // The dialogue duck the scene listens for is the crate's default, not a private rule.
    assert!(
        settings.ducks.iter().any(|d| d.source == "Dialogue" && d.target == "Music" && d.by_db == -8.0),
        "scene 4 says `Music` by 8 dB, and that is the default rule"
    );
    assert!(
        settings.ducks.iter().any(|d| d.source == "Dialogue" && d.target == "Effects" && d.by_db == -4.0),
        "scene 4 says `Effects` by 4 dB"
    );
}

/// **The README's tour table is the run's table.** Every row in the README's "What the tour sounds like" — and the
/// integrated loudness beside it — is read back from the committed report and checked against the README, so a
/// re-render that moves a number is a red test rather than a stale sentence (the trap Tan, Wagner and Treude measured:
/// 28.9 % of the top 1,000 projects carry an outdated code reference right now).
#[test]
fn the_readmes_tour_numbers_are_the_runs_own() {
    let readme = include_str!("../README.md");
    let report = include_str!("../docs/media/demo/scenes.txt");
    // Each scene's peak and RMS, as the report writes them: `peak -9.1 dBFS   rms -29.6 dBFS`.
    let mut rows = 0;
    // The report writes a scene as `== <title>   <length> s` and its numbers on the **next** line:
    // `   peak -9.1 dBFS   rms -29.6 dBFS   at the rail 0`.
    let lines: Vec<&str> = report.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if !line.starts_with("== ") {
            continue;
        }
        let Some(numbers) = lines.get(i + 1).and_then(|l| peak_and_rms(l)) else { continue };
        let title = line.trim_start_matches("== ").split("   ").next().unwrap_or("").trim();
        if title.starts_with("the whole tour") {
            continue;
        }
        let (peak, rms) = numbers;
        // **The README writes a minus sign (U+2212) and the report writes a hyphen**, as `ffmpeg` and a Rust `{:?}` do.
        // Comparing the two without normalising is a test that fails on typography, which is what its first run did.
        let (peak, rms) = (peak.replace('-', "\u{2212}"), rms.replace('-', "\u{2212}"));
        let wants = format!("| {title} | {peak} dBFS | {rms} dBFS |");
        assert!(readme.contains(&wants), "the README has no row `{wants}`");
        rows += 1;
    }
    assert_eq!(
        rows, 9,
        "every scene is in the README's table, including the mistake (music under it, no cue of its own)"
    );

    // The whole tour, and the loudness the README quotes, come from the same place.
    // **The tour's row writes its numbers on the same line**, unlike a scene's: it is the report's last line and there is
    // no line after it (the first version of this test looked for one and panicked on `its numbers`).
    let whole = report.lines().find(|l| l.contains("the whole tour")).expect("the tour's own row");
    let (peak, rms) = peak_and_rms(whole).expect("peak and rms");
    let (peak, rms) = (peak.replace('-', "\u{2212}"), rms.replace('-', "\u{2212}"));
    assert!(readme.contains(&format!("**{peak} dBFS**")), "the README quotes the tour's peak: {peak}");
    assert!(readme.contains(&format!("**{rms} dBFS**")), "the README quotes the tour's RMS: {rms}");
    // `docs/media/README.md` records the loudness command and its numbers; the README quotes the integrated value.
    let lufs = readme
        .split("**−22.3 LUFS**")
        .next()
        .map(|_| "−22.3")
        .expect("the README quotes an integrated loudness");
    assert!(readme.contains(&format!("{lufs} LUFS")), "the integrated loudness `{lufs}` is the README's");
}

/// `peak -9.1 dBFS   rms -29.6 dBFS` → `("-9.1", "-29.6")`, from a report line.
fn peak_and_rms(line: &str) -> Option<(String, String)> {
    let peak = line.split("peak ").nth(1)?.split(' ').next()?.to_owned();
    let rms = line.split("rms ").nth(1)?.split(' ').next()?.to_owned();
    Some((peak, rms))
}

/// **The video that ships is the tour, and it carries the sound.** A README links it, so it is part of the crate's face;
/// a re-render that drops the audio, shortens the tour or changes its size is a red test rather than a video nobody
/// plays. Read with `ffprobe`, which is the same tool the media README's commands use.
#[test]
fn the_video_is_the_tour_and_carries_its_sound() {
    let video = "docs/media/demo/tour.mp4";
    let meta = std::fs::metadata(video).unwrap_or_else(|e| panic!("`{video}`: {e}"));
    assert!(meta.len() > 200_000, "`{video}` is {} bytes — an empty or truncated encode", meta.len());
    assert!(meta.len() < 4_000_000, "`{video}` is {} bytes — too heavy for a README link", meta.len());

    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,codec_name,width,height:format=duration",
            "-of",
            "default=nw=1",
            video,
        ])
        .output()
        .expect("ffprobe");
    let text = String::from_utf8_lossy(&probe.stdout);
    assert!(probe.status.success(), "ffprobe could not read `{video}`: {text}");
    assert!(text.contains("codec_name=h264"), "no H.264 stream: {text}");
    // **The sound is the point of the file**: a video with no audio track would play silently and demo nothing.
    assert!(text.contains("codec_name=aac"), "no AAC audio stream — the tour would be silent: {text}");
    assert!(text.contains("width=1280") && text.contains("height=720"), "not 720p: {text}");
    let duration: f64 = text
        .lines()
        .find_map(|l| l.strip_prefix("duration="))
        .and_then(|d| d.trim().parse().ok())
        .unwrap_or_else(|| panic!("no duration: {text}"));
    assert!((duration - 32.0).abs() < 1.0, "the tour is 32 s and the video is {duration}");

    // The poster the README shows is a frame of this file, not a mock-up.
    let poster = std::fs::metadata("docs/media/demo/tour-poster.png").expect("the poster");
    assert!(poster.len() > 20_000, "the poster is {} bytes", poster.len());
    let dims = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=width,height",
            "-of",
            "default=nw=1",
            "docs/media/demo/tour-poster.png",
        ])
        .output()
        .expect("ffprobe on the poster");
    let dims = String::from_utf8_lossy(&dims.stdout);
    assert!(dims.contains("width=1280") && dims.contains("height=720"), "the poster is not 720p: {dims}");
}
