"""Every image the docs name must exist, and every image in docs/media must be named.

A README that shows a broken or stale picture is the front door of the crate, and nothing in `cargo test` looks at it.
This is the check the loop runs by hand; it is not part of the crate's suite because the crate does not ship `docs/`.
"""
import re, sys, pathlib

here = pathlib.Path(__file__).resolve()
root = next(p for p in here.parents if (p / "Cargo.toml").is_file())
readme = (root / "README.md").read_text()
media_readme = (root / "docs/media/README.md").read_text()
fail = 0

# 1. Every raw.githubusercontent URL in README.md names a file that exists.
for name in re.findall(r"raw\.githubusercontent\.com/[\w\-/]+/main/docs/media/([\w.\-]+)", readme):
    p = root / "docs/media" / name
    print(f"{'ok  ' if p.is_file() else 'MISS'}  README.md -> docs/media/{name}")
    fail += 0 if p.is_file() else 1

# 2. Every local image reference in docs/media/README.md exists.
for name in re.findall(r"!\[[^\]]*\]\(([\w.\-]+)\)", media_readme):
    p = root / "docs/media" / name
    print(f"{'ok  ' if p.is_file() else 'MISS'}  docs/media/README.md -> docs/media/{name}")
    fail += 0 if p.is_file() else 1

# 3. Every image file in docs/media is referenced by at least one of the two.
for p in sorted((root / "docs/media").glob("*")):
    if p.suffix in {".png", ".gif"}:
        named = p.name in readme or p.name in media_readme
        print(f"{'ok  ' if named else 'ORPH'}  docs/media/{p.name} is named in a README")
        fail += 0 if named else 1

print(f"\n{fail} problem(s)")
sys.exit(1 if fail else 0)
