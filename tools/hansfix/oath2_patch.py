"""HansFix: add six new oaths (indices 10-15) to SpEffectParam.

usage: python3 -I oath2_patch.py <tools_dir> <paramdef SpEffect.xml> <input regulation.bin> <output regulation.bin> <build.json>

IDs follow the tower convention 9600000 + index*10: +1 drawback, +3..+5 boon level I..III,
and for the status oath +6..+8 the timed buff its trigger rows start. Every other byte of the
regulation is verified unchanged.
"""
import sys, json, hashlib
sys.path.insert(0, sys.argv[1])
from erreg import Regulation, Param, load_def, get, put

DEF, SRC, OUT, BUILD = sys.argv[2:6]
FIRST = 9600000
ELEMENTS = ['magic', 'fire', 'thunder', 'dark']           # dark = holy in ER params
CUTS = ['slash', 'blow', 'thrust', 'neutral', 'magic', 'fire', 'thunder', 'dark']
TEMPLATE = 9600081          # clean resident tower row (only changeHpEstusFlaskCorrectRate set)
TRIGGER, BUFF = 321600, 321601  # Lord of Blood's Exultation: resident trigger -> 20 s buff

def base(index): return FIRST + index * 10

# index: (name, {offset: {field: value}}, template per offset)
RADIANT, IRON, BULWARK, GREED, PLAGUE, LIFELINE = 10, 11, 12, 13, 14, 15
LIFELINE_SECONDS = (10.0, 15.0, 20.0)
SPEC = {}
def plain(index, offset, fields):
    SPEC[base(index) + offset] = (TEMPLATE, dict(fields, changeHpEstusFlaskCorrectRate=1.0))

for lvl, rate in zip((3, 4, 5), (1.25, 1.40, 1.55)):
    plain(RADIANT, lvl, {f'{e}AttackRate': rate for e in ELEMENTS})
plain(RADIANT, 1, {'physicsAttackRate': 0.85})
for lvl, rate in zip((3, 4, 5), (1.15, 1.25, 1.35)):
    plain(IRON, lvl, {'physicsAttackRate': rate})
plain(IRON, 1, {f'{e}AttackRate': 0.75 for e in ELEMENTS})
for lvl, rate in zip((3, 4, 5), (0.90, 0.85, 0.80)):
    plain(BULWARK, lvl, {f'{c}DamageCutRate': rate for c in CUTS})
plain(BULWARK, 1, {'maxMpRate': 0.80})
for lvl, rate in zip((3, 4, 5), (1.25, 1.40, 1.60)):
    plain(GREED, lvl, {'haveSoulRate': rate})
plain(GREED, 1, {f'{c}DamageCutRate': 1.10 for c in CUTS})
for lvl, rate in zip((3, 4, 5), (1.15, 1.22, 1.30)):
    buff = base(PLAGUE) + lvl + 3
    SPEC[base(PLAGUE) + lvl] = (TRIGGER, {'cycleOccurrenceSpEffectId': buff, 'invocationConditionsStateChange1': 379,
                                          'invocationConditionsStateChange2': 380})
    SPEC[buff] = (BUFF, {f'atk{t}DmgCorrectRate_{e}': rate for t in ('Player', 'Enemy')
                         for e in ('Physics', 'Magic', 'Fire', 'Thunder', 'Dark')})
plain(PLAGUE, 1, {'registBloodChangeRate': 0.70, 'registPoizonChangeRate': 0.70, 'registDiseaseChangeRate': 0.70})
# Lifeline: resident no-death rows (+3..+5) while armed; on the first lethal hit the DLL swaps them for a
# game-timed row (+6..+8): no death, no damage, stamina use -90% for 10/15/20 s. Drawback: max HP -10%.
for lvl, seconds in zip((3, 4, 5), LIFELINE_SECONDS):
    plain(LIFELINE, lvl, {'noDead': 1})
    plain(LIFELINE, lvl + 3, dict({'noDead': 1, 'consumeStaminaRate': 0.10, 'effectEndurance': seconds},
                                  **{f'{c}DamageCutRate': 0.0 for c in CUTS}))
plain(LIFELINE, 1, {'maxHpRate': 0.90})
# r13 fallen-oath curses: 9600200 + kind*10 + (level-1) negative, +5 positive (the same effect reversed).
# kinds: 0 max HP, 1 magic/fire/lightning/holy damage taken, 2 max stamina, 3 stamina cost, 4 FP cost, 5 skill damage
CURSE_FIRST = 9600200
ELEMENT_FIRST = 9600260
SKILL_TEMPLATE = 9600001   # Steel oath drawback: attack rates limited to weapon skills via magicSubCategoryChange 112/111
LOW, HIGH = (0.08, 0.10, 0.12), (0.10, 0.12, 0.15)
for lvl in range(3):
    for positive in (False, True):
        off = lvl + (5 if positive else 0)
        lo, hi = LOW[lvl], HIGH[lvl]
        sign = -1 if positive else 1      # positive = the curse reversed
        SPEC[CURSE_FIRST + 0 + off] = (TEMPLATE, {'maxHpRate': 1 - sign * lo, 'changeHpEstusFlaskCorrectRate': 1.0})
        SPEC[CURSE_FIRST + 10 + off] = (TEMPLATE, dict({f'{c}DamageCutRate': 1 + sign * hi for c in ('magic', 'fire', 'thunder', 'dark')}, changeHpEstusFlaskCorrectRate=1.0))
        SPEC[CURSE_FIRST + 20 + off] = (TEMPLATE, {'maxStaminaRate': 1 - sign * lo, 'changeHpEstusFlaskCorrectRate': 1.0})
        SPEC[CURSE_FIRST + 30 + off] = (TEMPLATE, {'consumeStaminaRate': 1 + sign * lo, 'changeHpEstusFlaskCorrectRate': 1.0})
        SPEC[CURSE_FIRST + 40 + off] = (TEMPLATE, {f'{c}ConsumptionRate': 1 + sign * lo for c in ('magic', 'miracle', 'arts')} | {'changeHpEstusFlaskCorrectRate': 1.0})
        SPEC[CURSE_FIRST + 50 + off] = (SKILL_TEMPLATE, {f'{e}AttackRate': 1 - sign * hi for e in ('physics', 'magic', 'fire', 'thunder', 'dark')})
        # r20: damage taken from ONE random element per oath (magic / fire / lightning / holy)
        for e, elem in enumerate(('magic', 'fire', 'thunder', 'dark')):
            SPEC[ELEMENT_FIRST + e * 10 + off] = (TEMPLATE, {f'{elem}DamageCutRate': 1 + sign * hi, 'changeHpEstusFlaskCorrectRate': 1.0})

reg = Regulation(SRC)
fields, size = load_def(DEF)
entry = reg.bnd.find('SpEffectParam')
original = reg.bnd.data(entry)
param = Param(original, size)
before = {r[0]: r[1] for r in param.rows}
others = {e['name']: reg.bnd.data(e) for e in reg.bnd.entries if e is not entry}
existing = [rid for rid in SPEC if rid in before]
if existing and len(existing) != len(SPEC):
    raise SystemExit(f'new oath IDs partly in use: {existing}')

def build(rid):
    template, values = SPEC[rid]
    row = bytearray(before[template])
    for name, value in values.items():
        put(row, fields[name], value)
    return bytes(row)

if not existing:
    for rid in SPEC:
        param.rows.append([rid, build(rid), 'Tower fallen oath curse' if rid >= CURSE_FIRST else 'Tower oath HansFix6'])
    param.rows.sort(key=lambda r: r[0])
    reg.bnd.replace(entry, param.write())
    reg.save(OUT)
else:
    open(OUT, 'wb').write(open(SRC, 'rb').read())

# ---- verify by reading the output back ----
check = Regulation(OUT)
centry = check.bnd.find('SpEffectParam')
for e in check.bnd.entries:
    if e is not centry and check.bnd.data(e) != others[e['name']]:
        raise SystemExit(f'unrelated file changed: {e["name"]}')
cparam = Param(check.bnd.data(centry), size)
rows = {r[0]: r[1] for r in cparam.rows}
assert len(rows) == len(before) + (0 if existing else len(SPEC)), 'unexpected row count'
for rid, data in before.items():
    if rows[rid] != data:
        raise SystemExit(f'existing effect changed: {rid}')
for rid, (template, values) in SPEC.items():
    assert rows[rid] == build(rid), rid
    for name, value in values.items():
        got = get(rows[rid], fields[name])
        assert abs(got - value) < 1e-5, (rid, name, got, value)
    if 'effectEndurance' not in values:
        assert get(rows[rid], fields['effectEndurance']) == (20.0 if template == BUFF else -1.0), rid
ids = sorted(SPEC)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest().upper()
json.dump({'effects': ids, 'sourceRegulationSha256': sha(SRC), 'regulationSha256': sha(OUT)},
          open(BUILD, 'w'), indent=2)
print(f'verified {len(ids)} new oath effects; {len(before)} existing effects and {len(others)} other files unchanged')
