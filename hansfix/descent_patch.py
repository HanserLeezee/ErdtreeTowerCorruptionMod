"""HansFix wander descent: add enemy scaling rows and x2 / x-1 affix effect rows to SpEffectParam.

usage: python3 -I descent_patch.py <tools_dir> <paramdef SpEffect.xml> <input regulation.bin> <output regulation.bin> <build.json>

- 9641001..9641060: descent k -> maxHpRate 2*1.1^(k-1), *AttackPowerRate 1+0.05k (cloned from growth row 9640001)
- 9851000+kind*10+tier: affix effect 9850000+kind*10+tier with its value doubled
- 9852000+kind*10+tier: the same effect negated
- 9853000+kind*10+tier: the same effect x-2 (fallen boss / fallen shop bonus gear)
Every other byte of the regulation is verified unchanged.
"""
import sys, json, hashlib
sys.path.insert(0, sys.argv[1])
from erreg import Regulation, Param, load_def, get, put

DEF, SRC, OUT, BUILD = sys.argv[2:6]
AFFIX, DOUBLE, NEGATE, NEGATE2, ENEMY, GROWTH1 = 9850000, 9851000, 9852000, 9853000, 9641000, 9640001
ATK = [f'{e}AttackPowerRate' for e in ('physics', 'magic', 'fire', 'thunder', 'dark')]
HIT = [f'{e}AttackRate' for e in ('physics', 'magic', 'fire', 'thunder', 'dark')]
# kind -> (fields, value in percent (or points for kind 3), reduce?)
def kinds(t):
    return {
        0: (['maxHpRate'], 2 + t, False), 1: (['maxMpRate'], 3 + t, False), 2: (['maxStaminaRate'], 2 + t, False),
        3: (['staminaRecoverChangeSpeed'], 1 + t, False), 4: (ATK, 2 + t, False),
        5: (['magicConsumptionRate', 'miracleConsumptionRate'], 2 + t, True),
        6: ([f'{c}DamageCutRate' for c in ('slash', 'blow', 'thrust', 'neutral')], 1 + t * 0.5, True),
        7: (HIT, 6 + 2 * t, False), 8: (HIT, 8 + 3 * t, False), 9: (['toughnessDamageCutRate'], 5 + 2 * t, True),
        10: (['soulRate'], 5 + 3 * t, False), 11: ([f'{c}DamageCutRate' for c in ('magic', 'fire', 'thunder', 'dark')], 2 + t, True),
        12: (HIT, 6 + 2 * t, False), 13: (HIT, 6 + 2 * t, False), 14: (HIT, 8 + 2 * t, False), 15: (HIT, 5 + 2 * t, False),
        16: (HIT, 4 + 2 * t, False), 17: (HIT, 4 + 2 * t, False), 18: (['guardStaminaCutRate'], 8 + 3 * t, True),
        19: (['consumeStaminaRate'], 3 + t, True), 20: (['changeHpEstusFlaskCorrectRate', 'changeMpEstusFlaskCorrectRate'], 4 + 2 * t, False),
    }

reg = Regulation(SRC)
fields, size = load_def(DEF)
entry = reg.bnd.find('SpEffectParam')
param = Param(reg.bnd.data(entry), size)
before = {r[0]: r[1] for r in param.rows}
others = {e['name']: reg.bnd.data(e) for e in reg.bnd.entries if e is not entry}

SPEC = {}   # id -> (template id, {field: value}, name)
g = before[GROWTH1]
assert abs(get(g, fields['maxHpRate']) - 1.03) < 1e-5 and all(abs(get(g, fields[a]) - 1.015) < 1e-5 for a in ATK), 'growth template changed'
for k in range(1, 61):
    hp, atk = 2.0 * 1.1 ** (k - 1), 1.0 + 0.05 * k
    SPEC[ENEMY + k] = (GROWTH1, dict({'maxHpRate': hp}, **{a: atk for a in ATK}), f'Tower HansFix descent {k}')
for kind in range(21):
    if kind == 18: continue   # stateInfo 158 (talisman behaviour), not a number: never cursed
    for tier in range(4):
        src = AFFIX + kind * 10 + tier
        assert src in before, src
        names, value, reduce = kinds(tier)[kind]
        for name in names:
            got = get(before[src], fields[name])
            want = (1 + tier) if kind == 3 else 1 + (-value if reduce else value) / 100
            assert abs(got - want) < 1e-4, (src, name, got, want)
        for first, factor, label in ((DOUBLE, 2, 'x2'), (NEGATE, -1, 'x-1'), (NEGATE2, -2, 'x-2')):
            vals = {}
            for name in names:
                got = get(before[src], fields[name])
                vals[name] = got * factor if kind == 3 else 1 + (got - 1) * factor
            SPEC[first + kind * 10 + tier] = (src, vals, f'Tower HansFix affix {kind}:{tier} {label}')

existing = [rid for rid in SPEC if rid in before]
if existing:
    raise SystemExit(f'HansFix descent IDs already in use: {existing[:5]}')

def build(rid):
    template, values, _ = SPEC[rid]
    row = bytearray(before[template])
    for name, value in values.items():
        put(row, fields[name], value)
    return bytes(row)

for rid in SPEC:
    param.rows.append([rid, build(rid), SPEC[rid][2]])
param.rows.sort(key=lambda r: r[0])
reg.bnd.replace(entry, param.write())
reg.save(OUT)

check = Regulation(OUT)
centry = check.bnd.find('SpEffectParam')
for e in check.bnd.entries:
    if e is not centry and check.bnd.data(e) != others[e['name']]:
        raise SystemExit(f'unrelated file changed: {e["name"]}')
rows = {r[0]: r[1] for r in Param(check.bnd.data(centry), size).rows}
assert len(rows) == len(before) + len(SPEC)
for rid, data in before.items():
    if rows[rid] != data:
        raise SystemExit(f'existing effect changed: {rid}')
for rid, (template, values, _) in SPEC.items():
    assert rows[rid] == build(rid), rid
    for name, value in values.items():
        assert abs(get(rows[rid], fields[name]) - value) < 1e-4, (rid, name)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest().upper()
import os
json.dump({'effects': len(SPEC), 'sourceRegulationSha256': sha(SRC), 'regulationSha256': sha(OUT), 'regulationLength': os.path.getsize(OUT)},
          open(BUILD, 'w'), indent=2)
print(f'verified {len(SPEC)} new effects; {len(before)} existing effects and {len(others)} other files unchanged; size {os.path.getsize(OUT)}')
