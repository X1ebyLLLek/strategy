"""Builds the window's cursors: assets/ui/cursors/<theme>/<kind>.svg (64x64, drawn at 32 points).

    python -I tools/make_cursors.py

The shapes: our own arrow, I-beam and badges; the hands, the quill, the shield, the crate, the crossed
swords and the hourglass are game-icons.net outlines (CC BY 3.0, authors in assets/CREDITS.md) taken from
the game's sprite assets/ui/icons.svg. Each era its own metal: the staff's brass, the age of muskets'
ivory quill and sepia, the wastes' rust. Where the hand's point is (the hotspot) — HOT below, the same
numbers as `native/cursor.rs`.
"""
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SPRITE = open(os.path.join(ROOT, 'assets', 'ui', 'icons.svg'), encoding='utf-8').read()
SYM = dict(re.findall(r'<symbol id="i-([^"]+)"[^>]*>(.*?)</symbol>', SPRITE))

# the eras: the metal's gradient (light to dark), the outline, the highlight, the badges' ring
THEMES = {
    'staff': (['#fff4c8', '#e6c46a', '#b8893a', '#6e4a16'], '#22160a', '#fff8dc', '#d9b25a'),
    'parchment': (['#fffaf0', '#f1e4c4', '#d0b98c', '#957a4e'], '#2a1a0b', '#ffffff', '#c9a24a'),
    'rust': (['#f2b878', '#c46c34', '#8c4320', '#46220f'], '#150d07', '#ffd8a8', '#d89a2a'),
}

# where the hand's point is in each cursor (of 64)
HOT = {'arrow': (5, 4), 'hand': (17, 4), 'grab': (32, 32), 'attack': (5, 4), 'hold': (5, 4),
       'depot': (5, 4), 'text': (32, 32), 'wait': (32, 32)}

ARROW = 'M5 4 L5 49 L16.5 38.5 L25 57 L33.5 53 L25 35 L41 35 Z'


def head(theme):
    stops, edge, light, ring = THEMES[theme]
    grad = ''.join(f'<stop offset="{i / (len(stops) - 1):.2f}" stop-color="{c}"/>' for i, c in enumerate(stops))
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">'
        '<defs>'
        f'<linearGradient id="m" x1="0" y1="0" x2="0.9" y2="1">{grad}</linearGradient>'
        '<filter id="s" x="-30%" y="-30%" width="160%" height="160%">'
        '<feDropShadow dx="1.6" dy="2.6" stdDeviation="1.5" flood-color="#000" flood-opacity="0.55"/></filter>'
        '</defs>'
    )


def pits(theme, clip_id):
    """The wastes' metal: rust eaten in spots (clipped to the shape)."""
    if theme != 'rust':
        return ''
    spots = [(12, 14, 2.2), (9, 30, 1.6), (18, 40, 2.6), (27, 50, 1.8), (22, 30, 1.4), (34, 31, 1.2), (14, 22, 1.1),
             (40, 20, 2.0), (30, 12, 1.5), (46, 40, 2.4), (20, 52, 1.3), (38, 46, 1.7)]
    dots = ''.join(f'<circle cx="{x}" cy="{y}" r="{r}" fill="#2a1408" fill-opacity="0.55"/>' for x, y, r in spots)
    return f'<g clip-path="url(#{clip_id})">{dots}</g>'


def arrow(theme):
    stops, edge, light, ring = THEMES[theme]
    if theme == 'parchment':
        return quill(theme)
    return (
        f'<clipPath id="c"><path d="{ARROW}"/></clipPath>'
        f'<path d="{ARROW}" fill="url(#m)" stroke="{edge}" stroke-width="2.3" stroke-linejoin="round" filter="url(#s)"/>'
        + pits(theme, 'c') +
        f'<path d="M8 11 L8 41.5 L15.5 34.5" fill="none" stroke="{light}" stroke-opacity="0.75" stroke-width="1.5" stroke-linecap="round"/>'
    )


def icon(name, tr, theme, clip_id='ci', width=26):
    """A sprite outline under a transform: the metal, the outline, the rust."""
    stops, edge, light, ring = THEMES[theme]
    body = SYM[name]
    return (
        f'<clipPath id="{clip_id}"><g transform="{tr}">{body}</g></clipPath>'
        f'<g transform="{tr}" fill="url(#m)" stroke="{edge}" stroke-width="{width}" stroke-linejoin="round" filter="url(#s)">{body}</g>'
        + pits(theme, clip_id)
    )


def quill(theme):
    # the quill: the nib at the top left (the sprite's is at the top right: mirrored)
    s = 46 / 512
    tr = f'translate(4 4) scale({s:.5f}) translate(-11 -3) translate(512 0) scale(-1 1)'
    stops, edge, light, ring = THEMES[theme]
    return icon('pen', tr, theme, width=22) + f'<path d="M4 4 L9 8.5" stroke="#1a120a" stroke-width="2.6" stroke-linecap="round"/>'


def hand(theme):
    # the pointing hand, the finger straight up (the sprite's points down-right)
    s = 42 / 512
    tr = f'translate(17 4) scale({s:.5f}) translate(-175 0) translate(256 256) rotate(225) translate(-256 -256)'
    return icon('pointing', tr, theme)


def grab(theme):
    s = 44 / 512
    tr = f'translate(32 32) scale({s:.5f}) translate(-258 -255)'
    return icon('hand', tr, theme)


def badge(theme, name, fill):
    stops, edge, light, ring = THEMES[theme]
    s = 15 / 512
    return (
        f'<circle cx="46" cy="47" r="13" fill="{fill}" stroke="{ring}" stroke-width="2.2" filter="url(#s)"/>'
        f'<circle cx="46" cy="47" r="10.6" fill="none" stroke="#000" stroke-opacity="0.35" stroke-width="1"/>'
        f'<g transform="translate(38.5 39.5) scale({s:.5f})" fill="#fbf4e2">{SYM[name]}</g>'
    )


def text_beam(theme):
    stops, edge, light, ring = THEMES[theme]
    d = 'M24 14 L30 14 L32 16 L34 14 L40 14 L40 18 L34 18 L34 46 L40 46 L40 50 L34 50 L32 48 L30 50 L24 50 L24 46 L30 46 L30 18 L24 18 Z'
    return f'<path d="{d}" fill="url(#m)" stroke="{edge}" stroke-width="1.8" stroke-linejoin="round" filter="url(#s)"/>'


def wait(theme):
    s = 40 / 512
    tr = f'translate(32 32) scale({s:.5f}) translate(-256 -256)'
    return icon('hourglass', tr, theme)


def build(theme, kind):
    parts = {
        'arrow': lambda: arrow(theme),
        'hand': lambda: hand(theme),
        'grab': lambda: grab(theme),
        'attack': lambda: arrow(theme) + badge(theme, 'army', '#8e1c14'),
        'hold': lambda: arrow(theme) + badge(theme, 'shield', '#23466e'),
        'depot': lambda: arrow(theme) + badge(theme, 'crate', '#3d6326'),
        'text': lambda: text_beam(theme),
        'wait': lambda: wait(theme),
    }
    return head(theme) + parts[kind]() + '</svg>\n'


def main():
    n = 0
    for theme in THEMES:
        d = os.path.join(ROOT, 'assets', 'ui', 'cursors', theme)
        os.makedirs(d, exist_ok=True)
        for kind in HOT:
            open(os.path.join(d, f'{kind}.svg'), 'w', encoding='utf-8', newline='\n').write(build(theme, kind))
            n += 1
    print(n, 'cursors')


if __name__ == '__main__':
    main()
