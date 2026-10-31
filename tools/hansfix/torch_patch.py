"""HansFix torches: spell Ashes of War (torch only) and per-instance torch weapon slots.

usage: python3 -I torch_patch.py <tools_dir> <paramdef dir> <spells.tsv> <input regulation.bin> <output regulation.bin> <build.json>

For every spell i in scene-assets/hansfix-torch-spells.tsv:
  SwordArtsParam 9600+i     = copy of 10 ("no skill": the skill button does nothing); textId 9600+i (name injected at runtime)
  EquipParamGem 990000+i    = copy of 60300 with the spell's icon, skill 9600+i, mountable on torches only
  SpEffectParam 9600300+i   = 0.5 s marker that only plays the spell's cast SFX/SE on the player
  SpEffectVfxParam 9600300+i
Torch slots: EquipParamWeapon 24800000 + k*100 (k < SLOTS) = copies of the Torch (24000000). The DLL turns a slot into
one particular torch (base torch, scaling, affixes) at runtime, so every torch the player owns is its own row.
Every other row and file is verified unchanged.
"""
import sys, json, hashlib
sys.path.insert(0, sys.argv[1])
from erreg import Regulation, Param, load_def, get, put

DEFS, SPELLS, SRC, OUT, BUILD = sys.argv[2:7]
ART_FIRST, GEM_FIRST, FLASH_FIRST = 9600, 990000, 9600300
SLOT_FIRST, SLOTS, TORCH = 24800000, 512, 24000000
CAST_DMY = 220

spells = [l.rstrip('\n').split('\t') for l in open(SPELLS, encoding='utf-8') if l.strip() and not l.startswith('#')]
plan = {}   # stem -> (defn, [(id, template, values, name)])
def add(stem, defn, rid, template, values, name):
    plan.setdefault(stem, (defn, []))[1].append((rid, template, values, name))
for index, magic, name, icon, ref, kind, sfx in spells:
    i, icon, sfx = int(index), int(icon), int(sfx)
    add('SwordArtsParam', 'SwordArtsParam', ART_FIRST + i, 10, {'textId': ART_FIRST + i}, f'Tower torch spell {magic}')
    add('EquipParamGem', 'EquipParamGem', GEM_FIRST + i, 60300, {'iconId': icon, 'swordArtsParamId': ART_FIRST + i, 'defaultWepAttr': 0}, f'Tower torch ash {magic}')
    add('SpEffectParam', 'SpEffect', FLASH_FIRST + i, 9600081, {'effectEndurance': 0.5, 'vfxId': FLASH_FIRST + i}, f'Tower torch cast flash {magic}')
    add('SpEffectVfxParam', 'SpEffectVfx', FLASH_FIRST + i, 1660000, {'midstSfxId': -1, 'midstSeId': -1, 'midstDmyId': -1,
        'initSfxId': sfx, 'initSeId': sfx, 'initDmyId': CAST_DMY, 'finishSfxId': -1, 'finishSeId': -1}, f'Tower torch cast flash {magic}')
for k in range(SLOTS):
    add('EquipParamWeapon', 'EquipParamWeapon', SLOT_FIRST + k * 100, TORCH, {}, f'Tower torch slot {k}')

reg = Regulation(SRC)
files_before = {e['name']: reg.bnd.data(e) for e in reg.bnd.entries}
built = {}
for stem, (defn, rows_to_add) in plan.items():
    fields, size = load_def(f'{DEFS}/{defn}.xml')
    entry = reg.bnd.find(stem)
    param = Param(reg.bnd.data(entry), size)
    before = {r[0]: r[1] for r in param.rows}
    new = {}
    for rid, template, values, name in rows_to_add:
        assert rid not in before, f'{stem} {rid} already in use'
        row = bytearray(before[template])
        for field, value in values.items():
            put(row, fields[field], value)
        if stem == 'EquipParamGem':
            for field in fields:
                if field.startswith('canMountWep_'):
                    put(row, fields[field], 1 if field == 'canMountWep_Torch' else 0)
        param.rows.append([rid, bytes(row), name]); new[rid] = bytes(row)
    param.rows.sort(key=lambda r: r[0])
    reg.bnd.replace(entry, param.write())
    built[stem] = (size, before, new)
reg.save(OUT)

check = Regulation(OUT)
for e in check.bnd.entries:
    stem = e['name'].replace('\\', '/').split('/')[-1].rsplit('.', 1)[0]
    if stem not in built and check.bnd.data(e) != files_before[e['name']]:
        raise SystemExit(f'unrelated file changed: {e["name"]}')
for stem, (size, before, new) in built.items():
    rows = {r[0]: r[1] for r in Param(check.bnd.data(check.bnd.find(stem)), size).rows}
    assert len(rows) == len(before) + len(new), stem
    assert all(rows[k] == v for k, v in before.items()), f'{stem}: existing row changed'
    assert all(rows[k] == v for k, v in new.items()), f'{stem}: new row mismatch'
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest().upper()
json.dump({'spells': len(spells), 'slots': SLOTS, 'sourceRegulationSha256': sha(SRC), 'regulationSha256': sha(OUT)}, open(BUILD, 'w'), indent=2)
print(f'verified {len(spells)} torch spells and {SLOTS} torch slots; all other rows and files unchanged')
