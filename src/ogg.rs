//! **How long an Ogg Vorbis stream says it is**, and what a loop of it must be told.
//!
//! A Vorbis stream ends on a page whose granule position is the number of frames the stream has. The last packet decodes to a whole block, which is more, and the spec says the excess is discarded
//! (Vorbis I, A.2). **Neither of `rodio`'s Vorbis decoders discards it for every file**: on Kenney's 254 CC0 Ogg effects `lewton`, the one this crate uses, returned more than the stream declares for 51 — by 27 to
//! 936 frames, a median of 412 (9 ms), the loudest padding at −6 dBFS — and `symphonia`, built with `Decoder::new` as the mixer builds it, for 253, by up to 1 019 (`docs/loop/runs/2026-11-03-115/evidence/after/{ogg-loop-report,vorbis-backend-parity}.txt`). So **a loop
//! of an Ogg is longer than its stream by one padding at every wrap**: a gap or a repeated tail four to seven times what a listener can hear (about 3 ms), and a stem that drifts off its bar by that much a pass.
//! This module reads the length the stream declares from its pages, so each playback path can be told it: `PlaybackSettings::duration` on Bevy's (`bevy_audio` loops
//! `decoder.take_duration(duration).repeat_infinite()`), `take_duration` on the crate's own mixer's.
//!
//! **It refuses what it does not fully understand** — a chained or grouped file (more than one beginning-of-stream page), a file cut short, a first packet that is not Vorbis, a last page without the
//! end-of-stream flag, a granule position of −1 or 0, a stream the packet count cannot account for — and the caller then plays the file exactly as it always did.
//!
//! **Why a window and not a cap.** `semantic::decode` can check a declared length against the length it decoded; a playback path cannot. A well-formed stream of `N` audio packets decodes to between
//! `(N − 1)·short/2` and `(N − 1)·long/2` frames (each packet returns a quarter of the previous block and a quarter of its own) and declares at most one long block's padding less, so a granule outside
//! `((N − 1)·short/2 − long/2, (N − 1)·long/2]` cannot be this stream's.
//!
//! **Not detected, and named**: a stream that starts at a non-zero granule position (Vorbis I, A.2: *"a positive starting value simply indicates that this stream begins at some positive time offset"*)
//! declares its end *position*, not its length, and neither decoder applies the offset either, so the cost is a loop that is no better than the padding it already had.

use std::time::Duration;

/// The fixed part of an Ogg page header (RFC 3533 §6): `OggS`, version, flags, granule position, serial, sequence, CRC, segment count.
const PAGE_HEADER: usize = 27;

/// A Vorbis identification header (Vorbis I §4.2.2): `\x01vorbis`, version (4), channels (1), rate (4), three bitrates (12), block sizes (1), framing (1).
const IDENTIFICATION: usize = 30;

/// What a Vorbis stream's own pages say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stream {
    /// The stream's length in frames: the granule position of its last page.
    pub frames: u64,
    /// The sample rate its identification header names.
    pub sample_rate: u32,
    /// The channel count its identification header names.
    pub channels: u16,
}

/// The stream `bytes` is, or `None` when it is anything the walk does not fully understand.
///
/// One pass over the pages: each page's 27-byte header and segment table are read and its payload skipped by arithmetic, so the cost is a few reads a page — about 1 300 pages for a 5 MB file.
pub(crate) fn inspect(bytes: &[u8]) -> Option<Stream> {
    let (mut at, mut starts, mut packets) = (0usize, 0usize, 0u64);
    let (mut identification, mut last) = (None, None);
    while at < bytes.len() {
        let header = bytes.get(at..at.checked_add(PAGE_HEADER)?)?;
        if &header[..4] != b"OggS" || header[4] != 0 {
            return None;
        }
        let table_start = at + PAGE_HEADER;
        let table = bytes.get(table_start..table_start + usize::from(header[26]))?;
        let body_start = table_start + table.len();
        let end = body_start.checked_add(table.iter().map(|n| usize::from(*n)).sum::<usize>())?;
        let body = bytes.get(body_start..end)?; // a page that runs past the end of the file is a file cut short
        // A packet ends at a lacing value below 255; a larger one continues onto the next page.
        packets += table.iter().filter(|n| **n < 255).count() as u64;
        let flags = header[5];
        if flags & 0x02 != 0 {
            starts += 1;
            identification.get_or_insert(body); // the first stream's first packet
        } else if at == 0 {
            return None; // a file that does not begin with a beginning-of-stream page
        }
        last = Some((flags, i64::from_le_bytes(header[6..14].try_into().ok()?)));
        at = end;
    }
    let ((flags, granule), id) = (last?, identification?);
    // One logical stream (RFC 3533 §4: chaining and grouping both put a second beginning-of-stream page in the file), ended, with a position to end at.
    if starts != 1 || flags & 0x04 == 0 || granule <= 0 {
        return None;
    }
    if id.len() < IDENTIFICATION || &id[..7] != b"\x01vorbis" || id[7..11] != [0; 4] {
        return None; // not Vorbis, or not Vorbis I
    }
    let (channels, sample_rate) = (u16::from(id[11]), u32::from_le_bytes(id[12..16].try_into().ok()?));
    let (short, long) = (1u64 << (id[28] & 0x0f), 1u64 << (id[28] >> 4));
    if channels == 0
        || sample_rate == 0
        || !(64..=8192).contains(&short)
        || !(64..=8192).contains(&long)
        || short > long
    {
        return None;
    }
    // The window: the identification, comment and setup packets are not audio, and the first audio packet returns nothing.
    let decoding = packets.checked_sub(3)?.saturating_sub(1);
    let (frames, shortest, longest) = (granule as u64, decoding * short / 2, decoding * long / 2);
    if frames > longest || frames <= shortest.saturating_sub(long / 2) {
        return None;
    }
    Some(Stream { frames, sample_rate, channels })
}

/// The `Duration` that, handed to `PlaybackSettings::duration` (or to `take_duration` on the crate's mixer), plays **exactly** the stream's length — or `None` when [`inspect`] does not understand the stream.
pub(crate) fn loop_length(bytes: &[u8]) -> Option<Duration> {
    let stream = inspect(bytes)?;
    take_duration(stream.frames, stream.sample_rate, stream.channels)
}

/// The `Duration` that makes `rodio`'s `TakeDuration` emit exactly `frames` frames of `channels` samples at `sample_rate`.
///
/// **It is not `frames / rate`.** `TakeDuration` (`rodio-0.22.2/src/source/take.rs:59-126`) emits a sample while what remains is more than `p = ⌊10⁹ / (rate × channels)⌋` nanoseconds, and subtracts `p` after each.
/// `p` is truncated — by 6 to 8 parts in 10⁵ at 44.1 and 48 kHz stereo — so `frames / rate` is a few samples too long on half a second of audio and dozens on a few seconds. `N·p + 1` ns for `N` samples is
/// emitted `N` times: the remainders `N·p + 1, (N − 1)·p + 1, … p + 1` are each more than `p`, and the next, `1`, is not. That is `rodio`'s implementation, so a test with the real `TakeDuration` pins it.
pub(crate) fn take_duration(frames: u64, sample_rate: u32, channels: u16) -> Option<Duration> {
    let channels = u64::from(channels);
    let per_sample = 1_000_000_000u64.checked_div(u64::from(sample_rate) * channels).filter(|p| *p > 0)?;
    let samples = frames.checked_mul(channels)?;
    Some(Duration::from_nanos(samples.checked_mul(per_sample)?.checked_add(1)?))
}

/// **Ogg Vorbis streams made by hand**, for this module's tests and the mixer's: pages and an identification header built to the byte layout of RFC 3533 §6 and Vorbis I §4.2.2, with the parts a test
/// changes one at a time left open. They hold no audio — the packets are filler — and their CRC is zero, because nothing here verifies it and the decoder that does is not under test.
#[cfg(test)]
pub(crate) mod synthetic {

    /// The lacing values for one packet of `length` bytes: 255s, then the remainder (which may be 0, ending a packet whose length is a multiple of 255).
    fn lacing(length: usize) -> Vec<u8> {
        let mut values = vec![255u8; length / 255];
        values.push((length % 255) as u8);
        values
    }

    /// One Ogg page holding whole packets. The CRC is left zero: nothing here verifies it, and the decoder that does is not under test.
    fn page(flags: u8, granule: i64, serial: u32, sequence: u32, packets: &[Vec<u8>]) -> Vec<u8> {
        let table: Vec<u8> = packets.iter().flat_map(|p| lacing(p.len())).collect();
        let mut out = Vec::new();
        out.extend_from_slice(b"OggS");
        out.push(0); // version
        out.push(flags);
        out.extend_from_slice(&granule.to_le_bytes());
        out.extend_from_slice(&serial.to_le_bytes());
        out.extend_from_slice(&sequence.to_le_bytes());
        out.extend_from_slice(&[0; 4]); // CRC
        out.push(table.len() as u8);
        out.extend_from_slice(&table);
        for packet in packets {
            out.extend_from_slice(packet);
        }
        out
    }

    /// A Vorbis identification header (Vorbis I §4.2.2): 30 bytes, the block sizes as two exponents in one byte.
    fn identification(channels: u8, rate: u32, short_exponent: u8, long_exponent: u8) -> Vec<u8> {
        let mut packet = vec![0x01];
        packet.extend_from_slice(b"vorbis");
        packet.extend_from_slice(&0u32.to_le_bytes()); // version
        packet.push(channels);
        packet.extend_from_slice(&rate.to_le_bytes());
        packet.extend_from_slice(&[0; 12]); // the three bitrates
        packet.push(long_exponent << 4 | short_exponent);
        packet.push(1); // framing
        packet
    }

    /// The parts of a stream a test changes one at a time.
    #[derive(Clone)]
    pub(crate) struct Spec {
        pub(crate) channels: u8,
        pub(crate) rate: u32,
        pub(crate) short_exponent: u8,
        pub(crate) long_exponent: u8,
        /// Audio packets, after the three header packets.
        pub(crate) audio_packets: usize,
        /// The last page's granule position.
        pub(crate) granule: i64,
        /// The last page's flags: `0x04` is end-of-stream.
        pub(crate) last_flags: u8,
    }

    impl Spec {
        /// A well-formed stream of 100 audio packets at 48 kHz stereo, blocks of 256 and 2 048: it decodes to between 12 672 and 101 376 frames.
        pub(crate) fn good() -> Spec {
            Spec {
                channels: 2,
                rate: 48_000,
                short_exponent: 8,
                long_exponent: 11,
                audio_packets: 100,
                granule: 99_000,
                last_flags: 0x04,
            }
        }

        pub(crate) fn bytes(&self) -> Vec<u8> {
            let mut out = page(
                0x02,
                0,
                7,
                0,
                &[identification(self.channels, self.rate, self.short_exponent, self.long_exponent)],
            );
            out.extend(page(0, 0, 7, 1, &[b"\x03vorbis comment".to_vec(), b"\x05vorbis setup".to_vec()]));
            // The audio packets: all but the last on one page, the last alone on a page that ends the stream.
            let audio: Vec<Vec<u8>> = (0..self.audio_packets).map(|i| vec![i as u8; 40 + i]).collect();
            let (rest, last) = audio.split_at(audio.len().saturating_sub(1));
            if !rest.is_empty() {
                out.extend(page(0, 1_000, 7, 2, rest));
            }
            out.extend(page(self.last_flags, self.granule, 7, 3, last));
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::synthetic::Spec;
    use super::*;

    #[test]
    fn a_well_formed_stream_reports_its_declared_length_rate_and_channels() {
        let stream = inspect(&Spec::good().bytes()).expect("a well-formed stream is understood");
        assert_eq!(stream, Stream { frames: 99_000, sample_rate: 48_000, channels: 2 });
    }

    /// **Each guard is one row, and each row is the good stream with one thing wrong**, so a row that stops refusing is a guard that stopped existing.
    #[test]
    fn a_stream_the_walk_does_not_fully_understand_is_refused() {
        let good = Spec::good();
        assert!(inspect(&good.bytes()).is_some(), "the control: unmutated, it is understood");

        let one = good.bytes();
        let mut chained = one.clone();
        chained.extend(one.clone()); // two bos pages: chaining, or a grouped stream — no single length
        let mut cut = good.bytes();
        cut.truncate(cut.len() - 5); // the last page ends past the end of the file
        let mut garbage_between = good.bytes();
        garbage_between.splice(100..100, [0xde, 0xad]); // the pages no longer run unbroken
        let mut not_vorbis = good.bytes();
        not_vorbis[28..35].copy_from_slice(b"OpusHea"); // the first packet's marker is not `\x01vorbis`
        let mut other_version = good.bytes();
        other_version[35] = 1; // vorbis_version is not 0
        let mut zero_rate = good.bytes();
        zero_rate[40..44].copy_from_slice(&0u32.to_le_bytes());
        let mut no_start = good.bytes();
        no_start[5] = 0; // the first page is not marked beginning-of-stream

        let rows: Vec<(&str, Vec<u8>)> = vec![
            ("two beginning-of-stream pages", chained),
            ("a last page cut short", cut),
            ("bytes between pages", garbage_between),
            ("a first packet that is not Vorbis", not_vorbis),
            ("a Vorbis version other than 0", other_version),
            ("a sample rate of zero", zero_rate),
            ("a first page that is not a beginning", no_start),
            ("no end-of-stream flag on the last page", Spec { last_flags: 0, ..good.clone() }.bytes()),
            (
                "a granule of −1, `no packet finished on this page`",
                Spec { granule: -1, ..good.clone() }.bytes(),
            ),
            ("a granule of 0", Spec { granule: 0, ..good.clone() }.bytes()),
            ("zero channels", Spec { channels: 0, ..good.clone() }.bytes()),
            (
                "block sizes the wrong way round",
                Spec { short_exponent: 11, long_exponent: 8, ..good.clone() }.bytes(),
            ),
            ("a block size outside 2^6 … 2^13", Spec { long_exponent: 14, ..good.clone() }.bytes()),
            ("a stream with no audio packet", Spec { audio_packets: 0, ..good.clone() }.bytes()),
            ("not Ogg at all", b"RIFF....WAVEfmt ".to_vec()),
            ("nothing", Vec::new()),
            ("three bytes", b"Ogg".to_vec()),
        ];
        for (what, bytes) in rows {
            assert_eq!(inspect(&bytes), None, "refused: {what}");
        }
    }

    /// **The plausibility window.** A stream of `N` audio packets decodes to between `(N − 1)·short/2` and `(N − 1)·long/2` frames and declares at most one long block's
    /// padding less, so a granule outside `((N − 1)·short/2 − long/2, (N − 1)·long/2]` cannot belong to it: the length is refused and the loop left as it was. Its edges are accepted.
    #[test]
    fn a_granule_the_packet_count_cannot_hold_is_refused_and_its_edges_are_not() {
        let at = |granule: i64| inspect(&Spec { granule, ..Spec::good() }.bytes());
        let (high, low) = (99 * 2048 / 2, 99 * 256 / 2 - 2048 / 2); // 101 376 and 11 648
        assert!(at(high).is_some(), "the longest a 100-packet stream can decode to is accepted");
        assert_eq!(at(high + 1), None, "one frame more than it can decode to");
        assert!(at(low + 1).is_some(), "the shortest it can declare is accepted");
        assert_eq!(at(low), None, "one frame less than its padding allows");
        assert_eq!(at(100), None, "a granule of 100 for 100 packets is a wrong granule, not a short file");
    }

    #[test]
    fn a_stream_of_another_rate_and_one_channel_reports_both() {
        let mono = Spec { channels: 1, rate: 44_100, granule: 50_000, ..Spec::good() };
        assert_eq!(inspect(&mono.bytes()), Some(Stream { frames: 50_000, sample_rate: 44_100, channels: 1 }));
    }

    #[test]
    fn a_loop_is_told_a_length_only_for_a_stream_that_is_understood() {
        assert!(loop_length(&Spec::good().bytes()).is_some());
        assert_eq!(loop_length(&Spec { last_flags: 0, ..Spec::good() }.bytes()), None);
        assert_eq!(loop_length(b"RIFF"), None);
    }
}
