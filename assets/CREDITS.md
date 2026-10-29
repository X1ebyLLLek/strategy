Авторы графики игры

ЗНАЧКИ — game-icons.net, лицензия CC BY 3.0 (https://creativecommons.org/licenses/by/3.0/).
Взяты только контуры значков; цвет задаёт игра. Авторы (по файлам https://github.com/game-icons/icons):
- DarkZaitzev: revolt
- Delapouite: ancient-ruins, bandit, binoculars, bread, cardboard-box, castle, cavalry, coal-pile, coins-pile, factory, flag-objective, graduate-cap, grain-bundle, heavy-bullets, jerrycan, medicines, meeple-group, mortar, newspaper, old-wagon, powder-bag, revolver, ringing-alarm, rolled-cloth, shop, stamper, star-medal, truck, village, walkie-talkie, water-flask, wood-pile, wooden-crate
- Faithtoken: ore
- Guard13007: pause-button, play-button
- Lorc: anvil, back-forth, barbed-wire, battle-tank, broken-shield, bullets, cannon, cog, compass, crossed-pistols, crossed-swords, crown, erlenmeyer, falling-leaf, fist, gears, gem-pendant, hand, hazard-sign, horse-head, hourglass, hunting-horn, metal-bar, mining, mountains, pointing, quill, quill-ink, radar-dish, rocket, scroll-unfurled, snowflake-2, spade, spanner, sprout, sun, sword-clash, wheat
- Quoting: artillery-shell
- Sbed: shield
- Skoll: airplane, apc, brodie-helmet, jeep, musket

ШРИФТЫ — лицензия SIL Open Font License 1.1 (https://openfontlicense.org), файлы из сборок Fontsource:
- PT Serif, PT Mono, PT Sans Narrow — ParaType (Александра Корольков, Ольга Умпелева, Владимир Ефимов)
- Oswald — Vernon Adams, Kalapi Gajjar, Cyreal
- Old Standard TT — Алексей Крюков
- Cormorant SC — Christian Thalmann (Catharsis Fonts)
- Russo One — Jovanny Lemonad

ПОРТРЕТЫ, ФЛАГИ, ЗНАКИ ВОЙСК НА КАРТЕ, ФАКТУРЫ БУМАГИ И МЕТАЛЛА — нарисованы самой игрой (SVG, CSS).
ФИГУРЫ ВОЙСК НА КАРТЕ (солдаты, танки, орудия, всадники, машины) — из свободных моделей (Quaternius, CC0; пушка — CC-BY 3.0;
танк — «T-12 Soviet light tank lowpoly», artie31, CC0, OpenGameArt); переделаны: позы, оружие, головные уборы, окраска
(список — `assets/figures/CREDITS.txt`).

ЗДАНИЯ ГОРОДОВ НА КАРТЕ (дома, церковь, доходные дома) — свободные модели OpenGameArt (CC0 и CC-BY 3.0/4.0; авторы и ссылки —
`assets/buildings/CREDITS.txt`), запечены в треугольники одного цвета, цвета, масштаб и поворот изменены.

ЗЕМЛЯ НА КАРТЕ ОКНА (`assets/terrain/`) — фотофактуры ambientCG (https://ambientcg.com), лицензия CC0 1.0
(общественное достояние): трава Grass004, лес Moss002, холмы Ground037, горы Rock028, пустыня Ground059,
болото Ground036, тундра Ground068, снег Snow010A; цвет и карта нормалей, уменьшены до 512×512
(список — `assets/terrain/CREDITS.txt`).

ЛЕСА И БОЛОТА С ВЫСОТЫ (`assets/terrain/aerial_*.jpg`) — аэрофотоснимки USGS (The National Map), работы
правительства США — общественное достояние: лиственный лес (Озарк, Миссури), хвойный (север Мэна), болото (Сини,
Мичиган); вырезаны, сделаны бесшовными, выровнены по цвету (список — `assets/terrain/CREDITS.txt`).

РЕЛЬЕФ ГОР И ХОЛМОВ (`assets/terrain/relief_*.png`) — высоты USGS 3DEP (The National Map), общественное
достояние: горы — центральный Айдахо (Salmon River Mountains), холмы — плато Аллегейни (Пенсильвания); снят общий
наклон края, сделаны бесшовными (список — `assets/terrain/CREDITS.txt`).

ДОМА, ЗАВОДЫ, КОРА ДЕРЕВЬЕВ (`assets/objects/`) — фотофактуры ambientCG, CC0 1.0: черепица RoofingTiles008 и
RoofingTiles014A, сланец RoofingTiles002, солома ThatchedRoof001A, гофрожесть CorrugatedSteel005, кирпич Bricks059,
штукатурка Plaster001, доски WoodSiding008, камень Bricks076A, кора Bark012; цвет и карта нормалей, 512×512
(список — `assets/objects/CREDITS.txt`).

КУРСОРЫ ОКНА (`assets/ui/cursors/<эпоха>/`, собирает `tools/make_cursors.py`) — стрелка, I-образный
курсор и значки-кружки нарисованы игрой; рука-указатель (Lorc, pointing), открытая ладонь (Lorc, hand), перо (Lorc,
quill), скрещённые мечи (Lorc, crossed-swords), щит (Sbed, shield), ящик (Delapouite, wooden-crate), песочные часы
(Lorc, hourglass) — контуры game-icons.net, CC BY 3.0, залиты металлом эпохи.

ОКНО ИГРЫ: ПОЛОСЫ, ЛИСТЫ ПАПОК, РАМКИ (`assets/ui/`) — фотофактуры ambientCG, CC0 1.0, перекрашенные под эпоху:
штаб — Metal021, Paper003, Cardboard002; пергамент — Wood066, Paper003; ржавчина — Metal041B, Fabric030
(`assets/ui/CREDITS.txt`). Шрифты окна — те же OFL-шрифты (файлы TTF из репозитория Google Fonts).

Как добавить значок: скачать файл с game-icons.net (author_name.svg), вписать его в tools/make_icons.py и
запустить `python -I tools/make_icons.py ПАПКА_СО_ЗНАЧКАМИ` — пересоберёт assets/ui/icons.svg и список авторов
web/ui/icons_credits.txt (перенести его сюда).

## Портреты (`portraits/`)
- `g000`–`g346` — портреты генералов Военной галереи Зимнего дворца (Джордж Доу и его мастерская, 1819–1828),
  общественное достояние; взяты уменьшенные копии с Wikimedia Commons (категория «Portraits in the Military Gallery
  of the Winter Palace»), обрезаны до головы и плеч (`g155`–`g346` добавлены в 25-й сессии: остаток галереи без
  конных, в полный рост, гравюр и повторов). Источник и лицензия каждого файла — `portraits/SOURCES.txt`.
- `c000`–`c010` — мужские портреты из набора «public domain portraits» (qubodup, CC0, https://opengameart.org/node/3714),
  вырезанные из картин в общественном достоянии.
- `c011`–`c061` — мужские портреты русских художников XVIII–XIX веков (Д. Левицкий, В. Тропинин, К. Брюллов, В. Перов,
  И. Крамской), общественное достояние; с Wikimedia Commons (25-я сессия), обрезаны
  до головы и плеч по лицу. Источник каждого файла — `portraits/SOURCES.txt`.
