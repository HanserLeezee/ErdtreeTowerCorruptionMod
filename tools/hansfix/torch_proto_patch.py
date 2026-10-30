"""HansFix torch prototype: a torch-only Ash of War whose skill is an incantation, plus a cast-visual marker.

usage: python3 -I torch_proto_patch.py <tools_dir> <paramdef dir> <input regulation.bin> <output regulation.bin> <build.json>

Adds
  SwordArtsParam 9600      = copy of 10 ("no skill": pressing the skill button does nothing) named after the
                             spell (textId 603 = Golden Vow)
  EquipParamGem 990000     = copy of 60300 (Ash of War: Golden Vow) with the incantation's icon (goods 6600),
                             skill 9600, mountable on torches only, no default affinity
  SpEffectParam 9600300    = 0.5 s marker effect that only plays the cast visuals/sound on the player
  Bullet 10669000          = Golden Vow's area (10660000) widened to a 30 m radius
  SpEffectVfxParam 9600300 = start FX = the incantation's fire SFX (Magic 6600 fireSfxId 525631), start SE the same id
Every other row and file is verified unchanged.
"""
import sys, json, hashlib
sys.path.insert(0, sys.argv[1])
from erreg import Regulation, Param, load_def, get, put

DEFS, SRC, OUT, BUILD = sys.argv[2:6]
SPELL_ICON, CAST_SFX, CAST_DMY = 6504, 525631, 220   # [Incantation] Golden Vow: goods icon, Magic fireSfxId, body dummy

# stem in regulation, paramdef file, new id, template id, {field: value}
ROWS = [
    ('SwordArtsParam', 'SwordArtsParam', 9600, 10, {'textId': 603}),
    ('EquipParamGem', 'EquipParamGem', 990000, 60300, {'iconId': SPELL_ICON, 'swordArtsParamId': 9600, 'defaultWepAttr': 0}),
    ('SpEffectParam', 'SpEffect', 9600300, 9600081, {'effectEndurance': 0.5, 'vfxId': 9600300}),
    ('SpEffectVfxParam', 'SpEffectVfx', 9600300, 1660000, {'midstSfxId': -1, 'midstSeId': -1, 'midstDmyId': -1,
                                                           'initSfxId': CAST_SFX, 'initSeId': CAST_SFX, 'initDmyId': CAST_DMY,
                                                           'finishSfxId': -1, 'finishSeId': -1}),
    # 黄金誓言的范围判定放大到整个竞技场（半径 30 米），由 DLL 在玩家身上发射，队友（联机时）一并获得效果
    ('Bullet', 'BulletParam', 10669000, 10660000, {'hitRadius': 30.0}),
]
NAMES = {9600: 'Tower torch spell: Golden Vow', 990000: 'Tower torch ash: Golden Vow',
         9600300: 'Tower torch cast flash: Golden Vow', 10669000: 'Tower torch: Golden Vow (arena-wide)'}

reg = Regulation(SRC)
files_before = {e['name']: reg.bnd.data(e) for e in reg.bnd.entries}
touched, built = set(), {}
for stem, defn, rid, template, values in ROWS:
    fields, size = load_def(f'{DEFS}/{defn}.xml')
    entry = reg.bnd.find(stem)
    param = Param(reg.bnd.data(entry), size)
    rows = {r[0]: r[1] for r in param.rows}
    assert rid not in rows, f'{stem} {rid} already in use'
    row = bytearray(rows[template])
    for name, value in values.items():
        put(row, fields[name], value)
    if stem == 'EquipParamGem':
        for name in fields:
            if name.startswith('canMountWep_'):
                put(row, fields[name], 1 if name == 'canMountWep_Torch' else 0)
    param.rows.append([rid, bytes(row), NAMES[rid]]); param.rows.sort(key=lambda r: r[0])
    reg.bnd.replace(entry, param.write())
    touched.add(stem); built[(stem, rid)] = (defn, size, bytes(row), rows)
reg.save(OUT)

check = Regulation(OUT)
for e in check.bnd.entries:
    stem = e['name'].replace('\\', '/').split('/')[-1].rsplit('.', 1)[0]
    if stem not in touched and check.bnd.data(e) != files_before[e['name']]:
        raise SystemExit(f'unrelated file changed: {e["name"]}')
for (stem, rid), (defn, size, row, before) in built.items():
    rows = {r[0]: r[1] for r in Param(check.bnd.data(check.bnd.find(stem)), size).rows}
    assert len(rows) == len(before) + 1, stem
    assert all(rows[k] == v for k, v in before.items()), f'{stem}: existing row changed'
    assert rows[rid] == row, (stem, rid)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest().upper()
json.dump({'rows': [f'{s} {r}' for s, _, r, _, _ in ROWS], 'sourceRegulationSha256': sha(SRC), 'regulationSha256': sha(OUT)},
          open(BUILD, 'w'), indent=2)
print(f'verified {len(ROWS)} torch prototype rows; all other rows and files unchanged')
