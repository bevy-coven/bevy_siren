# research.md — iteration 59: an empty bus is not a bus mixed to the floor

## The item

`SirenMix::worst_peak_db(bus)` answers **`SILENT_DB`** for a bus with no voices. Its own doc says that is wrong:

> *"Nothing playing is not `SILENT_DB` — it is **no peak at all**, and `SILENT_DB` would read as a bus mixed into the
> floor rather than a bus with nothing on it."*

Then it returns `SILENT_DB` anyway. So the two facts the sentence distinguishes are **one number**:

| | `worst_peak_db` |
|---|---|
| a bus with **no voices** | `SILENT_DB` |
| a bus with 2 voices **mixed at the floor** | `SILENT_DB` |

**Measured**: `assert_ne!(empty, silent_with_voices)` **fails** — both read `−80.0`.

## Why this matters, and how much

A game checking `worst_peak_db(bus) > -1.0` for "safe" gets `true` on an empty bus, which is right by luck: an empty bus
is not hot. But a **meter** cannot tell them apart, and the panel proved it — `examples/mixer` filtered
`voices_on(bus) > 0` **before** asking, which is exactly the workaround a sentinel collision forces on every caller. The
workaround was in the crate's own example, so a consumer would write it too.

## Reachability, measured rather than assumed

Every caller in this repository filters first (`examples/mixer.rs`, `docs/guide/buses.md`), and the crate itself never
compares the two. So this is a **latent trap**, not an active wrong output — rank 3 (authoring ergonomics / what a host
must know), not rank 1. It is logged as that.

## Sources

**S1. `worst_peak_db`'s own doc** — *"nothing playing is not `SILENT_DB` — it is no peak at all"*. The API's own
sentence, contradicted by its own body. That is the whole finding.

**S2. `SILENT_DB`'s definition** (`src/model.rs`) — `-80.0`, the crate's floor for a bus mix. It is a *level*, so
borrowing it for *absence* is what makes the two collide. The crate already draws this distinction elsewhere:
`SirenMix::deepest_duck()` returns `Option<(&str, f32)>`, and `MusicState::blocked(layer)` returns `Option<usize>` —
`None` for "not this", a value for "this, with a number" is the crate's established shape.

**S3. Gregory §14.5.8 on budgets** — a bus is a group with a **level**, and a level exists whether or not a voice is on
it. "No voice" is a fact about the group's membership, not about its level; giving it the level of the floor conflates the
two.

**S4. Iteration 49's `worst_peak_db` doc** — *"every live voice counted at full scale, because the crate mixes samples and
never reads their amplitude — so a bus reading below 0 dBFS cannot clip and one reading above it may."* The bound is
about **voices**, and with none there is no bound, which is `None`.

## The technique

An **additive** method rather than a signature change:

```rust
pub fn peak_db(&self, bus: &str) -> Option<f32> {
    (self.voices_on(bus) > 0).then(|| self.worst_peak_db(bus))
}
```

`worst_peak_db` keeps its sentinel, because a caller that wants one number for every bus already has it and changing the
signature would break every consumer on a **published** crate (0.1.0, 9 downloads). `peak_db` is the method the doc was
always describing.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `SILENT_DB` | `−80.0` | S2 |
| a bus with no voices, `worst_peak_db` | `SILENT_DB` — collides | measured |
| a bus at `SILENT_DB` with 2 voices | `SILENT_DB` — the collision | measured |
| a bus at −70 dB with 2 voices | `−64.0`, above the floor, no collision | measured |
| a bus with no voices, `peak_db` | **`None`** | measured |
| callers filtering `voices_on > 0` before asking | **the example and the guide** | measured |

## What good sounds like

A meter asks `peak_db(bus)` and draws nothing for `None`, a bar for `Some(db)` — including a bar at the floor, which is
a real level. `worst_peak_db` is unchanged for a caller that wants the one-number form. Nothing audible changes.

## Blocked

`hs distill search` answered `500 … gpu busy` for the ninth iteration running; the sources are the crate's own doc and
Gregory's budget chapter in the vendored `docs/research/siren.md`. No DOI bears on a sentinel that contradicts its doc.
