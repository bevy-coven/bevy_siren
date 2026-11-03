//! **Every number the crate decides with, and no engine type.**
//!
//! Gains are decibels here and become a linear multiplier only where a sample path needs one, so a
//! whole mix is summed the way a mixer sums it rather than the way `f32` addition would: two sounds at
//! 0 dB make one at +6.02 dB, not one at 0 dB. [`SILENT_DB`] is the one floor — below it a gain is
//! silence and the arithmetic says `0.0`, never a number small enough to pass an "is it audible" check
//! and still cost a voice.
//!
//! Everything here is a pure function of its arguments, so `tests/model.rs` pins each one against a
//! number a person can check by hand, and the systems in the rest of the crate only choose which
//! function to call.

use std::collections::VecDeque;

use crate::cue::Attenuation;

/// Below this a gain is silence, not a small number. −80 dB is 1/10 000 of full scale.
pub const SILENT_DB: f32 = -80.0;

/// The fade a stop or a layer change takes when nobody timed it: short enough to read as immediate,
/// long enough not to click.
pub const DEFAULT_FADE_S: f32 = 0.12;

/// Every gain approaches its target with this time constant, so nothing steps audibly. A step is a
/// click, and a click is the sound of a program changing a number instead of a mixer ramping one.
pub const MIX_TAU_S: f32 = 0.05;

/// How close to a musical boundary, in whole boundaries, still counts as *on* it. At 120 bpm and four
/// beats a bar that is 2 µs, well under a frame: a frame that lands a rounding error late still
/// changes the music on the bar rather than skipping the bar.
const BOUNDARY_EPS: f64 = 1e-6;

/// A gain in dB as a linear multiplier: `10^(db / 20)`.
///
/// At or below [`SILENT_DB`], and for a `db` that is not finite, the answer is exactly `0.0` — the one
/// place silence is decided.
pub fn db_to_gain(db: f32) -> f32 {
    if !db.is_finite() || db <= SILENT_DB {
        return 0.0;
    }
    10f32.powf(db / 20.0)
}

/// A linear multiplier as dB: `20 * log10(|gain|)`.
///
/// A zero gain is [`SILENT_DB`], never negative infinity, and the answer is floored at [`SILENT_DB`]
/// for the same reason: a caller that writes this into a bus level must not be able to hand the next
/// multiplication a value that a comparison against silence misjudges.
pub fn gain_to_db(gain: f32) -> f32 {
    if !gain.is_finite() || gain.abs() <= db_to_gain(SILENT_DB) {
        return SILENT_DB;
    }
    let db = 20.0 * gain.abs().log10();
    db.max(SILENT_DB)
}

/// The dB of the linear sum of `parts`: the sound of several sources playing at once.
///
/// This is the reason the crate works in dB and sums in linear: two 0 dB sounds are one 6.02 dB sound,
/// and `-80 dB + -80 dB` is still silence rather than a doubling into audibility.
pub fn sum_db(parts: &[f32]) -> f32 {
    let linear: f32 = parts.iter().map(|db| db_to_gain(*db)).sum();
    gain_to_db(linear)
}

/// A caller's number, with one that is not a number read as `neutral`.
///
/// **One rule for every non-finite caller value the crate accepts.** [`distance_gain`] already does this - a `NaN`
/// distance is `0.0` m, the listener's feet, not a `NaN` volume that silences a bus (`docs/guide/spatial.md`) - and
/// [`chain_db`]'s silencing rule is right for a **bus level the crate computed**, not for a gain an author typed. A
/// non-finite gain used to reach `chain_db`, which returned [`SILENT_DB`]: the cue played **silent** while `Played`
/// reported `-80.00` as though that level were authored, which R6 forbids (*"a play is never silent: `Played` or
/// `Refused`, both with the reason"*).
pub fn finite_or(db: f32, neutral: f32) -> f32 {
    if db.is_finite() { db } else { neutral }
}

/// The dB of gains applied one after another: a cue's gain, then its slot's, then the bus's, then the
/// master's.
///
/// Chaining multiplies linear gains, which is adding dB — so this adds, and then applies the one floor
/// [`SILENT_DB`]: **any stage at or below it makes the whole chain silence**, because a silent stage
/// multiplied by anything is still silent. [`sum_db`] is its opposite — several sources heard at once —
/// and using that here would turn a cue at −80 dB back into an audible one.
pub fn chain_db(parts: &[f32]) -> f32 {
    let mut total = 0.0f32;
    for db in parts {
        if !db.is_finite() || *db <= SILENT_DB {
            return SILENT_DB;
        }
        total += *db;
    }
    total.max(SILENT_DB)
}

/// The loudness the audio backend adds by itself to a spatial voice heard from `emitter` between ears
/// at `left` and `right`, so the crate can divide it back out.
///
/// Bevy's and rodio's spatial path pans by distance: each ear's channel is multiplied by
/// `min(1, 1/d²) * (0.5 + ((d_this - d_other) / |left - right| + 1) / 4)`. That is a distance law as well
/// as a pan, and the crate's own [`distance_gain`] is the one a project writes in a cue table — so a
/// spatial voice's sink volume is `distance gain / backend gain`, and the sample a player hears follows
/// the cue's curve with the backend's stereo placement intact.
///
/// The formula is mirrored from `rodio-0.22.2/src/source/spatial.rs` (`Spatial::set_positions`), which
/// both backends' spatial path uses. `tests/model.rs` pins it at two geometries.
pub fn backend_spatial_gain(emitter: [f32; 3], left: [f32; 3], right: [f32; 3]) -> f32 {
    fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
        a.iter().zip(b.iter()).map(|(a, b)| (a - b) * (a - b)).sum()
    }
    let ear_gap = dist_sq(left, right).sqrt();
    if !ear_gap.is_finite() || ear_gap <= 0.0 {
        // The backend asserts two distinct ears; one ear is no pan, so no extra gain to divide out.
        return 1.0;
    }
    let left_sq = dist_sq(left, emitter).max(f32::MIN_POSITIVE);
    let right_sq = dist_sq(right, emitter).max(f32::MIN_POSITIVE);
    let left_d = left_sq.sqrt();
    let right_d = right_sq.sqrt();
    let left_pan = (((left_d - right_d) / ear_gap + 1.0) / 4.0 + 0.5).min(1.0);
    let right_pan = (((right_d - left_d) / ear_gap + 1.0) / 4.0 + 0.5).min(1.0);
    let left_gain = left_pan * (1.0 / left_sq).min(1.0);
    let right_gain = right_pan * (1.0 / right_sq).min(1.0);
    let average = (left_gain + right_gain) * 0.5;
    if average.is_finite() { average.clamp(f32::MIN_POSITIVE, 1.0) } else { 1.0 }
}

/// A pitch in semitones as a playback rate: `2^(semitones / 12)`, clamped to `1/16 ..= 16`.
///
/// The clamp is four octaves each way: past it a rate is a resampling artefact rather than a pitch, and
/// one that reaches zero or infinity breaks the decoder's span arithmetic instead of sounding wrong.
pub fn semitones_to_ratio(semitones: f32) -> f32 {
    if !semitones.is_finite() {
        return 1.0;
    }
    (2f32.powf(semitones / 12.0)).clamp(1.0 / 16.0, 16.0)
}

/// The semitones a playback rate actually plays at: the inverse of [`semitones_to_ratio`]'s clamp.
///
/// `semitones_to_ratio` clamps to `1/16 ..= 16`, so a detune past +/-48 semitones plays at the clamp rather than at the
/// number asked for. `Played.pitch` must report the **rate the device is at**, or the meter disagrees with the sound -
/// the one thing R6's second half forbids. Four octaves each way is the clamp's own span, not a new constant.
pub fn ratio_to_semitones(ratio: f32) -> f32 {
    if !ratio.is_finite() || ratio <= 0.0 {
        return 0.0;
    }
    12.0 * ratio.log2()
}

/// The semitones a detune plays at, after the clamp in [`semitones_to_ratio`].
pub fn played_semitones(semitones: f32) -> f32 {
    ratio_to_semitones(semitones_to_ratio(semitones))
}

/// The gain of a source `distance_m` from the listener, by the cue's [`Attenuation`].
///
/// `Inverse` is `near / (near + rolloff * (d - near).max(0))`: exactly `1.0` anywhere inside `near_m`,
/// then falling away as a physical point source does. `Linear` is `1 - (d / max).clamp(0, 1)`: exactly
/// `0.0` at and past `max_m`. `None` is `1.0`.
///
/// A distance that is not finite, or negative, is treated as `0.0`: a caller handing in a `NaN`
/// transform gets a sound at the listener's feet, not a `NaN` volume that silences the whole bus.
pub fn distance_gain(model: Attenuation, distance_m: f32) -> f32 {
    let d = if distance_m.is_finite() { distance_m.max(0.0) } else { 0.0 };
    match model {
        Attenuation::None => 1.0,
        Attenuation::Inverse { near_m, rolloff } => {
            let near = if near_m.is_finite() { near_m.max(f32::MIN_POSITIVE) } else { 1.0 };
            let rolloff = if rolloff.is_finite() { rolloff.max(0.0) } else { 0.0 };
            (near / (near + rolloff * (d - near).max(0.0))).clamp(0.0, 1.0)
        }
        Attenuation::Linear { max_m } => {
            let max = if max_m.is_finite() { max_m } else { 0.0 };
            if max <= 0.0 { 0.0 } else { (1.0 - (d / max)).clamp(0.0, 1.0) }
        }
    }
}

/// The duck `by_db` still holds `since_s` seconds after it started: full strength for `hold_s`, then a
/// linear recovery to 0 dB over `fade_s`.
///
/// `by_db` is negative for a duck. A `fade_s` of zero or less recovers at once after the hold — a duck a
/// game wants to be a step, not a ramp — and a `since_s` before the start is the full duck, because a
/// caller one frame early is not a reason to hear a duck leak.
pub fn active_duck_db(by_db: f32, since_s: f64, hold_s: f32, fade_s: f32) -> f32 {
    if !by_db.is_finite() || by_db >= 0.0 {
        return 0.0;
    }
    let since = if since_s.is_finite() { since_s.max(0.0) } else { 0.0 };
    let hold = if hold_s.is_finite() { f64::from(hold_s.max(0.0)) } else { 0.0 };
    if since < hold {
        return by_db;
    }
    let fade = if fade_s.is_finite() { f64::from(fade_s) } else { 0.0 };
    if fade <= 0.0 {
        return 0.0;
    }
    let through = ((since - hold) / fade).clamp(0.0, 1.0) as f32;
    let left = by_db * (1.0 - through);
    // A thousandth of a dB is nothing to hear, and float division lands a hair short of 0 at the end of the fade;
    // reporting exactly 0 makes "is this bus ducked" one comparison instead of an epsilon each caller guesses.
    if left > -1e-3 { 0.0 } else { left }
}

/// The time constant for `approach` to arrive within one part in a thousand of its target in `fade_s` seconds, or
/// [`MIX_TAU_S`] when `fade_s` is zero or less.
///
/// **A fade duration means "the time to arrive", everywhere in this crate.** [`active_duck_db`] reaches its target at
/// exactly `hold + fade`, and a music layer's [`fade_s`](crate::music::MusicLayers) is documented the same way. But
/// [`approach`] is exponential — it reaches 63.2 % of the way in one time constant and never quite arrives — so the
/// constant that honours that sentence is the one for which `1 - e^(-fade_s / tau) >= 1 - 1/1000`, i.e.
/// `tau = fade_s / ln(1000)`, about `fade_s / 6.9078`.
///
/// A thousandth of the move is below the crate's 0.01 dB comparison for any move up to 10 dB, and a caller with a
/// larger move snaps the last hundredth of a dB off anyway (`move_layers` and `update_mix` both do). A `fade_s` of
/// zero or less keeps [`MIX_TAU_S`], so "no fade given" and "the crate's easing" stay the same thing.
pub fn tau_for_fade(fade_s: f32) -> f32 {
    if !fade_s.is_finite() || fade_s <= 0.0 { MIX_TAU_S } else { fade_s / 1000f32.ln() }
}

/// How many low bits of a voice index name its **slot**, the rest its **generation**.
///
/// A mixer hands out an index per voice and three systems hold it across frames (a gain write, a stop fade, an
/// end-of-sample query), so an index must stay a valid name for exactly one voice. Slots are reused — the alternative
/// is keeping every voice a game has ever played — and the generation is what makes a reused slot a *different* name:
/// an index from a voice the world has not finished despawning addresses nothing, rather than addressing its
/// replacement.
pub const VOICE_SLOT_BITS: u32 = 32;

/// Packs a slot and its generation into one voice index: generation high, slot low.
pub fn pack_voice(slot: usize, generation: u32) -> usize {
    ((generation as usize) << VOICE_SLOT_BITS) | (slot & (usize::MAX >> (usize::BITS - VOICE_SLOT_BITS)))
}

/// Splits a voice index into its slot and generation, the inverse of [`pack_voice`].
pub fn unpack_voice(voice: usize) -> (usize, u32) {
    let slot = voice & (usize::MAX >> (usize::BITS - VOICE_SLOT_BITS));
    let generation = (voice >> VOICE_SLOT_BITS) as u32;
    (slot, generation)
}

/// A gain one frame closer to `target`: `current + (target - current) * (1 - exp(-dt / tau))`.
///
/// This is what makes a bus move without a click. A `tau` or a `dt` of zero or less returns `target`:
/// there is no smoothing to do with no time, and a caller that wants a step asks for one this way
/// rather than by special-casing it.
pub fn approach(current: f32, target: f32, tau: f32, dt: f32) -> f32 {
    if !current.is_finite() || !target.is_finite() {
        return target;
    }
    if tau <= 0.0 || dt <= 0.0 || !tau.is_finite() || !dt.is_finite() {
        return target;
    }
    current + (target - current) * (1.0 - (-dt / tau).exp())
}

/// The next musical boundary at or after `now_s`, for `tempo_bpm` quarter notes with `beats` per
/// boundary: `beats = 1` is the next beat, `beats = 4` the next bar.
///
/// A boundary already passed at `now_s` *is* the answer, so a frame that lands exactly on the bar
/// changes the music on that frame rather than on the next one. `tempo_bpm <= 0` or `beats <= 0` means
/// there is no beat grid and the answer is `now_s`, which makes every transition immediate.
pub fn next_boundary(now_s: f64, tempo_bpm: f32, beats: f64) -> f64 {
    if !tempo_bpm.is_finite() || tempo_bpm <= 0.0 || !beats.is_finite() || beats <= 0.0 || !now_s.is_finite()
    {
        return now_s;
    }
    let period = f64::from(60.0 / tempo_bpm) * beats;
    if !period.is_finite() || period <= 0.0 {
        return now_s;
    }
    let ratio = now_s.max(0.0) / period;
    let nearest = ratio.round();
    if (ratio - nearest).abs() < BOUNDARY_EPS {
        return nearest * period;
    }
    ratio.ceil() * period
}

/// Deterministic xorshift64, seeded once per simulation and threaded through every choice.
///
/// A game that wants its sounds reproducible — a replay, a capture, a test — seeds this and gets the
/// same sequence; one that does not still gets a sequence, because a `0` seed is mapped to a constant.
/// Never seeded from the clock inside this crate: a sound that cannot be reproduced cannot be debugged.
///
/// **The seed a stream came from is [`seed`](Self::seed)'s answer, not the one a caller asked for.** They differ for
/// `0`, which maps to [`ZERO_SEED`](Self::ZERO_SEED) — a value that is also `SirenSettings::default_seed()`, so
/// `seed: 0` and the default are **one stream**, and a report must say so rather than naming a number the stream did not
/// come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    /// The xorshift state, which changes on every draw.
    state: u64,
    /// The seed the state was built from, which never changes: **the number [`seed`](Self::seed) answers**, and the one
    /// a report, a log line and a panel must agree on.
    built_from: u64,
}

impl Rng {
    /// The state a zero seed maps to: xorshift's state must never be zero, or it produces zero forever.
    pub const ZERO_SEED: u64 = 0x5EED_51AE_0000_0001;

    /// A generator from `seed`. A zero seed is replaced by [`ZERO_SEED`](Self::ZERO_SEED).
    pub fn new(seed: u64) -> Self {
        let built_from = if seed == 0 { Self::ZERO_SEED } else { seed };
        Self { state: built_from, built_from }
    }

    /// The seed this stream was **built from** — `0` reported as [`ZERO_SEED`](Self::ZERO_SEED), which is the state it
    /// started at. It does not change as the stream advances.
    pub fn seed(&self) -> u64 {
        self.built_from
    }

    /// The next 32 bits.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    /// A number in `[0, 1)`, from the top 24 bits so every value is exactly representable in `f32`.
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u32() >> 8) as f32) / (1u32 << 24) as f32
    }
}

/// Which variant of a cue to play, without repeating one of the last few choices.
///
/// `avoid` is how many recent picks are refused: `1` never plays the same sample twice in a row, which
/// is what a footstep needs; a burst weapon sets `0` and takes the plain random draw. When every variant
/// is inside the avoided window — a cue with two sounds and `avoid: 4` — the most recent one is still
/// refused, because repeating it immediately is the one thing the window exists to prevent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantPicker {
    history: VecDeque<usize>,
    /// The **keys** heard, most recent last, for [`pick_from`](Self::pick_from). Kept apart from `history` so
    /// [`pick`](Self::pick) stays byte-identical for every caller that has no keys.
    heard: VecDeque<String>,
    /// **How many picks this picker remembers** — the field, not `VecDeque::capacity()`.
    ///
    /// `remember` used to bound itself by the deque's **allocator** capacity, which is a different number: `with_capacity(3)`
    /// may hand back more than three, and a picker built at `capacity 1` could never grow. The window a caller asked for
    /// is the window refused, so it is kept where it can be read and changed.
    window: usize,
}

impl Default for VariantPicker {
    /// One pick of memory: the same window [`Repeat::default`](crate::cue::Repeat::default) refuses.
    fn default() -> Self {
        Self::new(1)
    }
}

impl VariantPicker {
    /// A picker that remembers the last `capacity` picks.
    pub fn new(capacity: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(capacity),
            heard: VecDeque::with_capacity(capacity),
            window: capacity,
        }
    }

    /// **The window this picker refuses, changed to `capacity` picks.**
    ///
    /// [`pick_from`](Self::pick_from) takes `avoid` per call, so a rule read fresh from the cue can ask for a wider
    /// window than the picker was built with — a table reload that raises `Repeat::AvoidRepeat { avoid }`, or a cue that
    /// arrives with more variants. Without this the picker silently refuses only what its first play sized:
    /// `capacity 1, avoid 3` produced the **byte-identical** sequence to `capacity 1, avoid 1` over 24 picks.
    ///
    /// Growing reserves room; shrinking keeps the **most recent** picks, because the rule refuses what was heard
    /// recently and the oldest memory is the first thing a narrower window can no longer honour.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.window = capacity;
        self.history.reserve(capacity.saturating_sub(self.history.len()));
        self.heard.reserve(capacity.saturating_sub(self.heard.len()));
        while self.history.len() > capacity {
            self.history.pop_front();
        }
        while self.heard.len() > capacity {
            self.heard.pop_front();
        }
    }

    /// The window this picker refuses, in picks.
    pub fn capacity(&self) -> usize {
        self.window
    }

    /// **Picks a variant by what a player would hear**, or `None` when the cue has no variant to play.
    ///
    /// `keys[i]` is the `sound` slot `i` names. The avoided window is made of **keys**, not indices, so two slots naming
    /// one file are one candidate for repetition: after `"a"` is heard, every slot whose key is `"a"` is refused. That is
    /// the guarantee `Repeat::AvoidRepeat` makes — *"never one of the last `avoid` variants"* — and a player hears keys.
    ///
    /// [`pick`](Self::pick) is the same rule without keys and is unchanged; a picker that has never been given keys
    /// behaves exactly as it always did.
    pub fn pick_from(&mut self, keys: &[&str], avoid: u8, rng: &mut Rng) -> Option<usize> {
        let len = keys.len();
        if len == 0 {
            return None;
        }
        // **The memory grows to meet the rule it is asked to enforce**, so `avoid` means what it says however the
        // picker was built. It never shrinks here: a narrower `avoid` simply reads a shorter slice of the same memory,
        // and a caller that built a wider window keeps it.
        if usize::from(avoid) > self.window {
            self.set_capacity(usize::from(avoid));
        }
        if len == 1 {
            self.remember(0);
            self.remember_key(keys[0]);
            return Some(0);
        }
        let refused = usize::from(avoid).min(len);
        let recent: Vec<&str> = self.heard.iter().rev().take(refused).map(String::as_str).collect();
        let mut choices = Vec::with_capacity(len);
        for (i, key) in keys.iter().enumerate() {
            if !recent.contains(key) {
                choices.push(i);
            }
        }
        // Every key is inside the window: refuse only the sound heard immediately before this one.
        if choices.is_empty() {
            let last = self.heard.back().map(String::as_str);
            for (i, key) in keys.iter().enumerate() {
                if Some(*key) != last {
                    choices.push(i);
                }
            }
        }
        // A cue of one sound repeated has no alternative at all, and refusing it would be the crate inventing a
        // refusal for a table that is simply thin. It plays, and `problems()` is where the table should be named.
        if choices.is_empty() {
            choices.extend(0..len);
        }
        let at = ((rng.next_f32() * choices.len() as f32) as usize).min(choices.len() - 1);
        let picked = choices[at];
        self.remember(picked);
        self.remember_key(keys[picked]);
        Some(picked)
    }

    /// Picks a variant index in `0..len`, or `None` when the cue has no variant to play.
    pub fn pick(&mut self, len: usize, avoid: u8, rng: &mut Rng) -> Option<usize> {
        if len == 0 {
            return None;
        }
        if usize::from(avoid) > self.window {
            self.set_capacity(usize::from(avoid));
        }
        if len == 1 {
            self.remember(0);
            return Some(0);
        }
        let refused = usize::from(avoid).min(len);
        let recent: Vec<usize> = self.history.iter().rev().take(refused).copied().collect();
        let mut choices = Vec::with_capacity(len);
        for i in 0..len {
            if !recent.contains(&i) {
                choices.push(i);
            }
        }
        // Every variant is inside the window: refuse only the pick immediately before this one.
        if choices.is_empty() {
            let last = self.history.back().copied();
            for i in 0..len {
                if Some(&i) != last.as_ref() {
                    choices.push(i);
                }
            }
        }
        let at = ((rng.next_f32() * choices.len() as f32) as usize).min(choices.len() - 1);
        let picked = choices[at];
        self.remember(picked);
        Some(picked)
    }

    /// Forgets every pick, so the next one is unconstrained.
    pub fn reset(&mut self) {
        self.history.clear();
        self.heard.clear();
    }

    fn remember_key(&mut self, key: &str) {
        if self.window == 0 {
            return;
        }
        if self.heard.len() >= self.window {
            self.heard.pop_front();
        }
        self.heard.push_back(key.to_owned());
    }

    fn remember(&mut self, index: usize) {
        if self.window == 0 {
            return;
        }
        if self.history.len() >= self.window {
            self.history.pop_front();
        }
        self.history.push_back(index);
    }
}

/// One live voice, as the limiter sees it: its `index` in the caller's list is what is returned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    /// The caller's index for this voice.
    pub index: usize,
    /// Its cue's priority; lower is stolen first.
    pub priority: u8,
    /// When it started, in seconds on the simulation's clock; the older of two equal priorities goes
    /// first, so the newest sound is the one a player hears keep playing.
    pub start_s: f64,
    /// A voice the voice budget may never take: the music, a line of dialogue, a stinger.
    pub unstealable: bool,
}

/// The voices to stop so that `need` more fit inside `max`: lowest priority first, then oldest first,
/// and never one marked `unstealable`.
///
/// **An empty answer means nothing has to be stopped *or* that the room cannot be made.** The caller
/// tells the two apart with `candidates.len() + need > max`, and refuses the request in the second case
/// rather than breaking its own budget — a game that says 32 voices gets 32, not 33 with a warning.
pub fn steal(candidates: &[Candidate], need: usize, max: usize) -> Vec<usize> {
    let room = max.saturating_sub(candidates.len());
    if room >= need {
        return Vec::new();
    }
    let wanted = need - room;
    let mut stealable: Vec<&Candidate> = candidates.iter().filter(|c| !c.unstealable).collect();
    if stealable.len() < wanted {
        return Vec::new();
    }
    stealable.sort_by(|a, b| {
        a.priority.cmp(&b.priority).then(a.start_s.total_cmp(&b.start_s)).then(a.index.cmp(&b.index))
    });
    stealable.into_iter().take(wanted).map(|c| c.index).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// The ensemble: combining signals into one score, and explaining it.
// ---------------------------------------------------------------------------------------------------------------
//
// An ensemble that cannot be explained is indistinguishable from a magic number, so every contribution is kept rather
// than summed away. This lives here, not in `semantic/`, because `tests/all.rs`'s engine-free scan covers this file: a
// score that needed an `App` to test would be a score nobody tests.

/// Which signal a [`Contribution`] is about, so a breakdown is read by name rather than by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Signal {
    /// Embedding similarity: what the words matched.
    Semantic,
    /// Acoustic agreement: loudness, brightness and noise floor.
    Acoustic,
    /// Agreement on tempo, when both are known.
    Tempo,
    /// Agreement on key, when both are known.
    Key,
}

/// Every signal, in the order a breakdown lists them.
pub const SIGNALS: [Signal; 4] = [Signal::Semantic, Signal::Acoustic, Signal::Tempo, Signal::Key];

impl Signal {
    /// The name a breakdown prints, which is also the field name a project's weights file uses.
    pub fn name(self) -> &'static str {
        match self {
            Signal::Semantic => "semantic",
            Signal::Acoustic => "acoustic",
            Signal::Tempo => "tempo",
            Signal::Key => "key",
        }
    }

    /// Where this signal sits in a [`Breakdown`]'s array.
    pub fn index(self) -> usize {
        match self {
            Signal::Semantic => 0,
            Signal::Acoustic => 1,
            Signal::Tempo => 2,
            Signal::Key => 3,
        }
    }
}

/// **One signal's contribution to a result**, with its raw value, its weight and what it contributed.
///
/// The three are kept separate because a caller reading a result needs them separately: the raw value says *what the
/// signal saw*, the weight says *how much the project cared*, and the contribution is the product. A result that only
/// carried the product would leave "why is this first" unanswered whenever a weight is small.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contribution {
    /// Which signal this is.
    pub signal: Signal,
    /// The signal's raw value, in its own units.
    pub raw: f32,
    /// The weight the project gave this signal.
    pub weight: f32,
    /// `raw * weight`, which is what entered the sum.
    pub contribution: f32,
    /// True when the signal had no value for this pair — a key that is unknown, a tempo that was not estimated.
    ///
    /// **A missing signal is not a zero.** A zero contribution from a signal that had nothing to say and a zero from
    /// one that was consulted and disagreed are different facts, and a breakdown that conflates them misleads exactly
    /// the person reading it to understand the ranking.
    pub missing: bool,
}

impl Contribution {
    /// **A signal that was consulted.** A non-finite value or weight contributes nothing and is marked missing rather
    /// than poisoning the sum with a NaN — the same rule [`db_to_gain`] follows for a gain.
    pub fn scored(signal: Signal, raw: f32, weight: f32) -> Contribution {
        let usable = raw.is_finite() && weight.is_finite() && weight >= 0.0;
        Contribution {
            signal,
            raw,
            weight,
            contribution: if usable { raw * weight } else { 0.0 },
            missing: !usable,
        }
    }

    /// **A signal that had no value for this pair**, so it contributes nothing and says so.
    pub fn absent(signal: Signal, weight: f32) -> Contribution {
        Contribution { signal, raw: f32::NAN, weight, contribution: 0.0, missing: true }
    }
}

/// **Everything one result was scored from**, in a fixed order, so two runs compare field for field.
#[derive(Debug, Clone, PartialEq)]
pub struct Breakdown {
    /// One entry per signal, indexed by [`Signal::index`].
    pub parts: [Contribution; 4],
}

impl Breakdown {
    /// Build a breakdown. The parts are placed by their own `signal`, so the array order cannot be got wrong.
    pub fn new(parts: [Contribution; 4]) -> Breakdown {
        let mut ordered = [
            Contribution::absent(Signal::Semantic, 0.0),
            Contribution::absent(Signal::Acoustic, 0.0),
            Contribution::absent(Signal::Tempo, 0.0),
            Contribution::absent(Signal::Key, 0.0),
        ];
        for p in parts {
            ordered[p.signal.index()] = p;
        }
        Breakdown { parts: ordered }
    }

    /// **The score**: the sum of every contribution.
    ///
    /// A sum rather than a mean, because the weights *are* the normalisation: a project that halves a weight is saying
    /// that signal matters half as much, and dividing by the total would undo that. [`Breakdown::shares`] is the
    /// normalised view for reporting.
    pub fn score(&self) -> f32 {
        self.parts.iter().map(|p| p.contribution).sum()
    }

    /// The contribution about one signal.
    pub fn part(&self, signal: Signal) -> Contribution {
        self.parts[signal.index()]
    }

    /// **Each signal's share of the score**, for a result's "why did this match".
    ///
    /// Returns `None` when the score is zero or not finite: there is no share of nothing, and four divisions by zero
    /// would be four NaNs a caller then has to guard.
    pub fn shares(&self) -> Option<[f32; 4]> {
        let total = self.score();
        if total == 0.0 || !total.is_finite() {
            return None;
        }
        let mut shares = [0.0f32; 4];
        for (i, p) in self.parts.iter().enumerate() {
            shares[i] = p.contribution / total;
        }
        Some(shares)
    }

    /// **The strongest signal that counted for the result, and the strongest that counted against it.**
    ///
    /// The pair a person reads first: *"matched on semantic, penalised for loudness"*. `None` when nothing counted, so a
    /// caller cannot report a reason that does not exist. Ties go to the earlier signal in [`SIGNALS`], so two runs
    /// cannot disagree.
    pub fn strongest(&self) -> Option<(Contribution, Option<Contribution>)> {
        let mut best: Option<Contribution> = None;
        let mut worst: Option<Contribution> = None;
        for p in &self.parts {
            if p.missing || p.contribution == 0.0 {
                continue;
            }
            if p.contribution > 0.0 && best.is_none_or(|b| p.contribution > b.contribution) {
                best = Some(*p);
            }
            if p.contribution < 0.0 && worst.is_none_or(|w| p.contribution < w.contribution) {
                worst = Some(*p);
            }
        }
        best.map(|b| (b, worst))
    }

    /// **A sentence a panel or a log can show**, naming what carried the result and what counted against it.
    ///
    /// The same shape this crate's refusals use: the thing, then what it was. A breakdown a person has to decode is a
    /// breakdown that does not explain anything.
    pub fn explain(&self) -> String {
        let missing: Vec<&str> =
            SIGNALS.into_iter().filter(|s| self.part(*s).missing).map(Signal::name).collect();

        let Some((best, worst)) = self.strongest() else {
            let mut s = String::from("no signal contributed");
            if !missing.is_empty() {
                s.push_str(&format!("; no value for {}", missing.join(", ")));
            }
            return s;
        };

        let mut s = format!(
            "{} carried it ({:+.3} from {} raw at weight {})",
            best.signal.name(),
            best.contribution,
            best.raw,
            best.weight
        );
        if let Some(w) = worst {
            s.push_str(&format!(
                "; {} counted against it ({:+.3} from {} raw)",
                w.signal.name(),
                w.contribution,
                w.raw
            ));
        }
        if !missing.is_empty() {
            s.push_str(&format!("; no value for {}", missing.join(", ")));
        }
        s
    }
}
