# Night lighting for the arena maps (堕落模式)

Every tower room is in one of the three colosseum maps (m45_00 / m45_01 / m45_02). Their lighting
files (`/param/drawparam/m45_0X_0000.gparam.dcx`) have a single time-of-day key, so changing the
game clock never changed the lighting there. Open-world files such as `m60_00_0000` carry keys
0, 4, 8, 12, 15, 21, 22.5 (key = game hour - 6, wrapping), and the engine blends them by the clock.

`night.py` rewrites each arena gparam with the same keys: 0-12 keep the original (day) values, and
15 / 21 / 22.5 get a night version derived from Limgrave (m60_00_0000) — each float value is scaled
by Limgrave's night/day ratio (22:00 vs 10:00), angles take Limgrave's night angle, and LightSet
colour intensities are dimmed a further x0.6. The DLL already sets the clock to 22:00 at the red
campfire and during the fallen challenge, and the maps reset to 07:00 on every load, so all other
rooms keep their day lighting.

Steps: `export-gparam.ps1` (run on Windows next to the game; read-only) pulls the vanilla files out of
Data0.bdt using `bhd.py`'s offsets; `night.py` builds the new gparams (`gpwrite.py` round-trips the
GPARAM format byte-for-byte); `oz.c` (mingw, run with the game's oo2core_6_win64.dll) compresses
them to Kraken DCX. Output goes to `scene-assets/param/drawparam/`.

r14: m45_02 (Limgrave colosseum) is not shipped — its night gparam made the game abort on load (fail-fast 7) while m45_00/m45_01 load fine; cause still unknown.

r18: none of the night gparams are shipped any more. After the v101 port the game also aborted (same
fail-fast 7 site) while loading m45_00 with the night file, which had loaded fine on v99. Editing the
colosseum gparams is not reliable; the patch ships the vanilla files for all three so unpacking over
an older patch restores them. Night for the fallen challenge needs another approach (e.g. EMEVD
`Activate Gparam Override` with a vanilla night gparam sub-id).
