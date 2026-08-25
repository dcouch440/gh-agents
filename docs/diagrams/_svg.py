#!/usr/bin/env python3
"""Shared SVG primitives for the README diagram generators."""

import textwrap
from html import escape
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "images"

# ── palette ────────────────────────────────────────────────────────────────
BG        = "#0f1319"
CARD      = "#161b22"
CARD2     = "#12171e"
BORDER    = "#2b333d"
BORDER_HI = "#3d4854"
FG        = "#e6edf3"
MUTED     = "#8b949e"
DIM       = "#6e7781"
TAG       = "#7ee787"   # xml tags
GREEN     = "#3fb950"
BLUE      = "#58a6ff"
AMBER     = "#d29922"
PURPLE    = "#bc8cff"

MONO = "ui-monospace,SFMono-Regular,'SF Mono',Menlo,Consolas,'Liberation Mono',monospace"
SANS = "-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif"

CH = 0.602  # monospace advance width as a fraction of font-size


def esc(s):
    return escape(s, quote=False)


class Canvas:
    def __init__(self, w):
        self.w = w
        self.parts = []
        self.maxy = 0

    def add(self, s, bottom=None):
        self.parts.append(s)
        if bottom is not None:
            self.maxy = max(self.maxy, bottom)

    def rect(self, x, y, w, h, fill=CARD, stroke=BORDER, rx=6, sw=1, dash=None):
        d = f' stroke-dasharray="{dash}"' if dash else ""
        self.add(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" '
                 f'fill="{fill}" stroke="{stroke}" stroke-width="{sw}"{d}/>', y + h)

    def text(self, x, y, s, size=12, fill=FG, family=MONO, weight="normal",
             anchor="start", opacity=1.0):
        self.add(f'<text x="{x}" y="{y}" font-family="{family}" font-size="{size}" '
                 f'font-weight="{weight}" fill="{fill}" text-anchor="{anchor}" xml:space="preserve" '
                 f'opacity="{opacity}">{esc(s)}</text>', y)

    def path(self, d, stroke=BORDER_HI, sw=1.6, marker=True, dash=None):
        m = ' marker-end="url(#arrow)"' if marker else ""
        dd = f' stroke-dasharray="{dash}"' if dash else ""
        self.add(f'<path d="{d}" fill="none" stroke="{stroke}" stroke-width="{sw}"'
                 f' stroke-linejoin="round" stroke-linecap="round"{m}{dd}/>')

    def render(self, h):
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{self.w}" height="{h}" '
            f'viewBox="0 0 {self.w} {h}" font-family="{SANS}">\n'
            f'<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" '
            f'markerWidth="6" markerHeight="6" orient="auto-start-reverse">'
            f'<path d="M0,0 L10,5 L0,10 z" fill="{BORDER_HI}"/></marker></defs>\n'
            f'<rect width="{self.w}" height="{h}" fill="{BG}"/>\n'
            + "\n".join(self.parts) + "\n</svg>\n"
        )


def wrap(s, width):
    """Wrap preserving blank lines and existing hard breaks."""
    out = []
    for para in s.split("\n"):
        if not para.strip():
            out.append("")
        else:
            out.extend(textwrap.wrap(para, width) or [""])
    return out


def chars_for(w, size, pad=11):
    return int((w - pad * 2) / (size * CH))


def block(c, x, y, w, lines, size=11.5, lh=15.5, pad=11,
          fill=CARD2, stroke=BORDER, title=None, title_fill=MUTED, title_size=10):
    """Two-pass bordered monospace block: measure, draw rect, then draw text."""
    n = len(lines) + (1 if title else 0)
    h = pad * 2 + n * lh + (3 if title else 0)
    c.rect(x, y, w, h, fill=fill, stroke=stroke)
    ty = y + pad + size
    if title:
        c.text(x + pad, ty, title, size=title_size, fill=title_fill, weight="bold")
        ty += lh + 3
    for txt, col in lines:
        if txt:
            c.text(x + pad, ty, txt, size=size, fill=col)
        ty += lh
    return y + h


def xml_lines(raw, width, tagcol=TAG, txtcol=FG):
    """Colour <tag> lines green, body text default."""
    out = []
    for ln in wrap(raw, width):
        s = ln.strip()
        if s.startswith("<") and s.endswith(">") and " " not in s.strip("<>/"):
            out.append((ln, tagcol))
        elif s.startswith("<") and s.endswith(">"):
            out.append((ln, tagcol))
        else:
            out.append((ln, txtcol))
    return out


