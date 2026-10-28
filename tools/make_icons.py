"""Builds assets/ui/icons.svg (one sprite of <symbol>s) and assets/ui/icons_credits.txt from game-icons.net.

Icons are CC BY 3.0 by their authors (https://game-icons.net). Only the outline (path data) of each
icon is kept; the sprite colours them with currentColor. Ids are `i-<name>`.

    python -I tools/make_icons.py DIR_WITH_DOWNLOADED_SVGS
(each downloaded file named author_name.svg, as fetched from
https://raw.githubusercontent.com/game-icons/icons/master/<author>/<name>.svg)
"""
import os, re, sys

# our name -> author/icon
ICONS = {
    # the spheres of the state
    'econ': 'delapouite/factory',
    'science': 'lorc/erlenmeyer',
    'army': 'lorc/crossed-swords',
    'news': 'delapouite/newspaper',
    'scroll': 'lorc/scroll-unfurled',
    'alarm': 'delapouite/ringing-alarm',
    # how the country lives
    'people': 'delapouite/meeple-group',
    'fist': 'lorc/fist',
    'revolt': 'darkzaitzev/revolt',
    'grain': 'delapouite/grain-bundle',
    'coins': 'delapouite/coins-pile',
    'gears': 'lorc/gears',
    'anvil': 'lorc/anvil',
    'supply': 'delapouite/heavy-bullets',
    'binoculars': 'delapouite/binoculars',
    'graduate': 'delapouite/graduate-cap',
    'stamper': 'delapouite/stamper',
    'quill': 'lorc/quill-ink',
    'castle': 'delapouite/castle',
    'medal': 'delapouite/star-medal',
    'compass': 'lorc/compass',
    'flag': 'delapouite/flag-objective',
    'wire': 'lorc/barbed-wire',
    'backforth': 'lorc/back-forth',
    'pause': 'guard13007/pause-button',
    'play': 'guard13007/play-button',
    # kinds of troops (a pack names one for each unit)
    'helmet': 'skoll/brodie-helmet',
    'tank': 'lorc/battle-tank',
    'shell': 'quoting/artillery-shell',
    'mortar': 'delapouite/mortar',
    'plane': 'skoll/airplane',
    'radar': 'lorc/radar-dish',
    'musket': 'skoll/musket',
    'horn': 'lorc/hunting-horn',
    'cavalry': 'delapouite/cavalry',
    'cannon': 'lorc/cannon',
    'bandit': 'delapouite/bandit',
    'jeep': 'skoll/jeep',
    'apc': 'skoll/apc',
    'rocket': 'lorc/rocket',
    # the navy (26th session)
    'anchor': 'lorc/anchor',
    'ship': 'lorc/galleon',
    'sub': 'delapouite/submarine',
    # the window: map modes, the country's bar, cursors, battles
    'mountains': 'lorc/mountains',
    'mining': 'lorc/mining',
    'ruins': 'delapouite/ancient-ruins',
    'crate': 'delapouite/wooden-crate',
    'crown': 'lorc/crown',
    'pointing': 'lorc/pointing',
    'hand': 'lorc/hand',
    'shield': 'sbed/shield',
    'hourglass': 'lorc/hourglass',
    'pen': 'lorc/quill',
    'clash': 'lorc/sword-clash',
    'broken': 'lorc/broken-shield',
    'village': 'delapouite/village',
    # the date's season, the alerts under the country's bar
    'sun': 'lorc/sun',
    'snow': 'lorc/snowflake-2',
    'sprout': 'lorc/sprout',
    'leaf': 'lorc/falling-leaf',
    'hazard': 'lorc/hazard-sign',
    # goods of the markets (a pack names one for each good)
    'wheat': 'lorc/wheat',
    'bread': 'delapouite/bread',
    'cloth': 'delapouite/rolled-cloth',
    'jerrycan': 'delapouite/jerrycan',
    'coal': 'delapouite/coal-pile',
    'ore': 'faithtoken/ore',
    'metal': 'lorc/metal-bar',
    'box': 'delapouite/cardboard-box',
    'shop': 'delapouite/shop',
    'pistols': 'lorc/crossed-pistols',
    'bullets': 'lorc/bullets',
    'wood': 'delapouite/wood-pile',
    'horse': 'lorc/horse-head',
    'gem': 'lorc/gem-pendant',
    'powder': 'delapouite/powder-bag',
    'water': 'delapouite/water-flask',
    'cog': 'lorc/cog',
    'meds': 'delapouite/medicines',
    'revolver': 'delapouite/revolver',
    # companies of support (a pack names one for each)
    'spade': 'lorc/spade',
    'radio': 'delapouite/walkie-talkie',
    'truck': 'delapouite/truck',
    'spanner': 'lorc/spanner',
    'wagon': 'delapouite/old-wagon',
    # the orders: plans of the groups, the chosen divisions, the clock
    'strike': 'lorc/striking-arrows',
    'wall': 'delapouite/stone-wall',
    'staff': 'delapouite/organigram',
    'stop': 'delapouite/stop-sign',
    'back': 'lorc/return-arrow',
    'rally': 'lorc/rally-the-troops',
    'route': 'delapouite/path-distance',
    'march': 'lorc/boot-prints',
    'faster': 'delapouite/fast-forward-button',
    'slower': 'delapouite/fast-backward-button',
    'target': 'lorc/on-target',
    # the order of society, the ledger (23rd session)
    'faith': 'lorc/holy-symbol',
    'vote': 'delapouite/vote',
    'chart': 'delapouite/histogram',
    'balance': 'lorc/scales',
    'forecast': 'lorc/crystal-ball',
    'pie': 'delapouite/pie-chart',
}

AUTHORS = {
    'lorc': 'Lorc', 'delapouite': 'Delapouite', 'skoll': 'Skoll', 'darkzaitzev': 'DarkZaitzev',
    'guard13007': 'Guard13007', 'quoting': 'Quoting', 'sbed': 'Sbed', 'faithtoken': 'Faithtoken',
}

src = sys.argv[1]
here = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'assets', 'ui')
symbols, credits = [], {}
for ours, theirs in ICONS.items():
    author, name = theirs.split('/')
    text = open(os.path.join(src, f'{author}_{name}.svg'), encoding='utf-8').read()
    ds = [d for d in re.findall(r'<path[^>]*\sd="([^"]+)"', text) if d != 'M0 0h512v512H0z']
    assert ds and all(re.fullmatch(r'[0-9MmLlHhVvCcSsQqTtAaZz.,\- e]+', d) for d in ds), theirs
    symbols.append(f'<symbol id="i-{ours}" viewBox="0 0 512 512">' + ''.join(f'<path d="{d}"/>' for d in ds) + '</symbol>')
    credits.setdefault(AUTHORS[author], []).append(name)
sprite = '<svg xmlns="http://www.w3.org/2000/svg" fill="currentColor">\n' + '\n'.join(symbols) + '\n</svg>\n'
open(os.path.join(here, 'icons.svg'), 'w', encoding='utf-8', newline='\n').write(sprite)
lines = [f'- {who}: {", ".join(sorted(names))}' for who, names in sorted(credits.items())]
open(os.path.join(here, 'icons_credits.txt'), 'w', encoding='utf-8', newline='\n').write('\n'.join(lines) + '\n')
print(len(symbols), 'icons;', len(sprite), 'bytes')
