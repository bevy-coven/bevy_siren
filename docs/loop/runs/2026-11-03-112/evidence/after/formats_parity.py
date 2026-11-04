"""Does the crate's decoder agree with ffmpeg's, on real MP3 and Ogg Vorbis files, through the whole pipeline?

A *control* library is the same files decoded by a reference decoder — `ffmpeg`, or `libvorbis` (libsndfile) for Ogg — to float32 WAV at each
file's own rate and channels (the crate reads WAV exactly: symphonia's PCM reader is a copy). `examples/semantic` is run on both with the
same descriptions and the similarity it prints for every file is compared. Everything after the decoder — the downmix, the resampler, the
mel spectrogram, the audio tower, the text tower — is the same code on both sides, so a difference is the decoder's alone.

    python3 formats_parity.py <lossy-dir> <wav-control-dir> "<phrase>" ["<phrase>" ...]

Needs the example built (`cargo build --release --features semantic --example semantic`) and SIREN_MODEL, SIREN_VOCAB,
SIREN_AUDIO_MODEL and SIREN_MODEL_VERSION set (`tools/export_clap_onnx.py` makes the files). The example prints three decimals, so the
comparison is to 5 x 10^-4 and a difference below that reads as zero.
"""
import os
import re
import subprocess
import sys

EXAMPLE = os.path.join(os.path.dirname(os.path.abspath(__file__)), *[".."] * 6, "target", "release", "examples", "semantic")
LINE = re.compile(r"^\s*\d+\. (\S+)\s+score \S+\s+similarity (\S+)", re.M)


def ranked(library, phrase):
    """Every file the example returns for `phrase`, as {stem: similarity}. `--k 100` is its ceiling, above every library here."""
    run = subprocess.run([EXAMPLE, library, phrase, "--k", "100", "--no-play"], capture_output=True, text=True)
    if run.returncode != 0 and "nothing in this library matched" not in run.stdout:
        sys.exit("the example failed on %s for %r:\n%s" % (library, phrase, run.stderr[-600:]))
    # Exit 1 with "nothing in this library matched" is the example saying every similarity was below its 0.0 floor: an empty table.
    return {os.path.splitext(m.group(1))[0]: float(m.group(2)) for m in LINE.finditer(run.stdout)}


def spearman(a, b, names):
    """Rank correlation of two similarity tables over `names` (ties broken by name, which only matters at three decimals)."""
    def ranks(t):
        order = sorted(names, key=lambda n: (-t[n], n))
        return {n: i for i, n in enumerate(order)}
    ra, rb, n = ranks(a), ranks(b), len(names)
    return 1.0 - 6.0 * sum((ra[x] - rb[x]) ** 2 for x in names) / (n * (n * n - 1))


def main():
    lossy, control, phrases = sys.argv[1], sys.argv[2], sys.argv[3:]
    files = len([f for f in os.listdir(lossy) if f.lower().endswith((".mp3", ".ogg", ".flac"))])
    print("%s: %d files, against the control library %s" % (os.path.basename(lossy.rstrip("/")), files, os.path.basename(control.rstrip("/"))))
    print("%-30s %5s %9s %9s %9s %10s %7s %s" % ("phrase", "pairs", "max diff", "mean diff", "identical", "one-sided", "rho", "top 3 the same"))
    worst, total, same_total, top_ok, one_sided, rhos = 0.0, 0, 0, 0, 0, []
    for phrase in phrases:
        a, b = ranked(lossy, phrase), ranked(control, phrase)
        common = sorted(set(a) & set(b))
        # The example's default floor is 0.0 and drops a negative cosine, so a file near zero can be kept on one side only. It is
        # counted, with the value the keeping side printed (the dropping side's was negative): a difference of at least that much.
        stray = {n: (a.get(n) if n in a else b.get(n)) for n in set(a) ^ set(b)}
        if len(common) < 3:
            print("%-30s %5d   (fewer than three files kept on both sides: not comparable; %d kept on one side only)" % (phrase, len(common), len(stray)))
            continue
        diffs = [abs(a[n] - b[n]) for n in common]
        top_a = sorted(common, key=lambda n: (-a[n], n))[:3]
        top_b = sorted(common, key=lambda n: (-b[n], n))[:3]
        # Order is only comparable where the printed values differ; two files tied at three decimals may swap.
        same_top = [round(a[n], 3) for n in top_a] == [round(b[n], 3) for n in top_b]
        rho = spearman(a, b, common)
        worst = max(worst, max(diffs), max(stray.values(), default=0.0))
        total += len(diffs)
        same = sum(1 for d in diffs if d < 5.0e-4)
        same_total += same
        top_ok += same_top
        one_sided += len(stray)
        rhos.append(rho)
        print("%-30s %5d %9.1e %9.1e %4d/%-4d %4d (%.3f) %7.4f %s"
              % (phrase, len(diffs), max(diffs), sum(diffs) / len(diffs), same, len(diffs), len(stray), max(stray.values(), default=0.0), rho, same_top))
    print("\nworst difference: %.1e;  %d of %d pairs equal to the printed precision;  %d files kept on one side only;  "
          "top three equal for %d of %d phrases;  rank correlation min %.4f, mean %.4f"
          % (worst, same_total, total, one_sided, top_ok, len(phrases), min(rhos), sum(rhos) / len(rhos)))


if __name__ == "__main__":
    main()
