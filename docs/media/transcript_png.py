"""Render a terminal transcript to the PNG a README shows.

    python3 docs/media/transcript_png.py docs/media/semantic.txt docs/media/semantic.png

Needs Pillow and macOS's Menlo, the monospace the other stills were captured with. It draws text and nothing else: a line that
starts with `$` is the command, a line that starts with `Played` or `Refused` is the crate's answer, and the rest is the example's
own output. Nothing is added, wrapped or cut, so the picture is exactly the transcript.
"""
import sys

from PIL import Image, ImageDraw, ImageFont

FONT = "/System/Library/Fonts/Menlo.ttc"
SIZE = 15
LEADING = 5
MARGIN = 28
BACKGROUND = (27, 29, 35)
COLOURS = {"command": (152, 195, 121), "answer": (229, 192, 123), "text": (216, 222, 233), "quiet": (140, 148, 163)}


def colour_of(line):
    if line.startswith("$"):
        return COLOURS["command"]
    if line.startswith(("Played", "Refused", "done")):
        return COLOURS["answer"]
    if line.startswith("     ") and not line.startswith("      "):
        return COLOURS["quiet"]  # the reason and the provenance under a result
    return COLOURS["text"]


def main():
    source, target = sys.argv[1], sys.argv[2]
    lines = open(source, encoding="utf-8").read().rstrip("\n").split("\n")
    font = ImageFont.truetype(FONT, SIZE)
    width = max(int(font.getlength(line)) for line in lines) + 2 * MARGIN
    row = SIZE + LEADING
    image = Image.new("RGB", (width, len(lines) * row + 2 * MARGIN), BACKGROUND)
    draw = ImageDraw.Draw(image)
    for i, line in enumerate(lines):
        draw.text((MARGIN, MARGIN + i * row), line, font=font, fill=colour_of(line))
    image.save(target, optimize=True)
    print("%s: %dx%d, %d lines" % (target, image.width, image.height, len(lines)))


if __name__ == "__main__":
    main()
