"""HansFix torch prototype: add one torch-only Ash of War whose skill is an incantation.

usage: python3 -I torch_proto_patch.py <tools_dir> <paramdef dir> <input regulation.bin> <output regulation.bin> <build.json>

Adds
  SwordArtsParam 9600  = copy of 10 ("no skill": pressing the skill button does nothing) with the
                         spell's skill name (textId 603 = Golden Vow)
  EquipParamGem 990000 = copy of 60300 (Ash of War: Golden Vow) with the incantation's icon (goods 6600),
                         skill 9600, mountable on torches only, no default affinity
Every other row and file is verified unchanged.
"""
import sys, json, hashlib
sys.path.insert(0, sys.argv[1])
from erreg import Regulation, Param, load_def, get, put

DEFS, SRC, OUT, BUILD = sys.argv[2:6]
ART, ART_TEMPLATE, ART_TEXT = 9600, 10, 603
GEM, GEM_TEMPLATE = 990000, 60300
SPELL_ICON = 6504   # EquipParamGoods 6600 [Incantation] Golden Vow

reg = Regulation(SRC)
others_before = {e['name']: reg.bnd.data(e) for e in reg.bnd.entries}

def load(stem):
    fields, size = load_def(f'{DEFS}/{stem}.xml')
    entry = reg.bnd.find(stem)
    return fields, size, entry, Param(reg.bnd.data(entry), size)

art_f, art_size, art_e, arts = load('SwordArtsParam')
gem_f, gem_size, gem_e, gems = load('EquipParamGem')
art_before = {r[0]: r[1] for r in arts.rows}
gem_before = {r[0]: r[1] for r in gems.rows}
assert ART not in art_before and GEM not in gem_before, 'prototype ids already in use'

art = bytearray(art_before[ART_TEMPLATE]); put(art, art_f['textId'], ART_TEXT)
gem = bytearray(gem_before[GEM_TEMPLATE])
put(gem, gem_f['iconId'], SPELL_ICON)
put(gem, gem_f['swordArtsParamId'], ART)
put(gem, gem_f['defaultWepAttr'], 0)
for name in gem_f:
    if name.startswith('canMountWep_'):
        put(gem, gem_f[name], 1 if name == 'canMountWep_Torch' else 0)
arts.rows.append([ART, bytes(art), 'Tower torch spell: Golden Vow']); arts.rows.sort(key=lambda r: r[0])
gems.rows.append([GEM, bytes(gem), 'Tower torch ash: Golden Vow']); gems.rows.sort(key=lambda r: r[0])
reg.bnd.replace(art_e, arts.write())
reg.bnd.replace(gem_e, gems.write())
reg.save(OUT)

check = Regulation(OUT)
for e in check.bnd.entries:
    stem = e['name'].replace('\\', '/').split('/')[-1].rsplit('.', 1)[0]
    if stem not in ('SwordArtsParam', 'EquipParamGem') and check.bnd.data(e) != others_before[e['name']]:
        raise SystemExit(f'unrelated file changed: {e["name"]}')
for stem, size, before, rid, built in (('SwordArtsParam', art_size, art_before, ART, bytes(art)), ('EquipParamGem', gem_size, gem_before, GEM, bytes(gem))):
    rows = {r[0]: r[1] for r in Param(check.bnd.data(check.bnd.find(stem)), size).rows}
    assert len(rows) == len(before) + 1, stem
    assert all(rows[k] == v for k, v in before.items()), f'{stem}: existing row changed'
    assert rows[rid] == built, stem
g = {r[0]: r[1] for r in Param(check.bnd.data(check.bnd.find('EquipParamGem')), gem_size).rows}[GEM]
assert get(g, gem_f['iconId']) == SPELL_ICON and get(g, gem_f['swordArtsParamId']) == ART and get(g, gem_f['canMountWep_Torch']) == 1
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest().upper()
json.dump({'gem': GEM, 'art': ART, 'sourceRegulationSha256': sha(SRC), 'regulationSha256': sha(OUT)}, open(BUILD, 'w'), indent=2)
print(f'verified torch prototype gem {GEM} / art {ART}; all other rows and files unchanged')
