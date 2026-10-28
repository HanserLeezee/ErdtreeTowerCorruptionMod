# HansFix regulation tools

`oath2_patch.py` adds the five HansFix6 oaths (灵辉/锻铁/磐石/贪婪/疫血, kinds 10–14) to
`SpEffectParam` in `scene-assets/regulation.bin` (IDs 9600101–9600148). `erreg.py` is a small
Python reader/writer for the encrypted regulation (AES → DCX ZSTD → BND4 → PARAM) that re-serializes
an unchanged param byte-for-byte; the patch verifies every other file and every existing row unchanged.

    pip install pycryptodome zstandard
    python3 -I oath2_patch.py tools/hansfix <Paramdex>/ER/Defs/SpEffect.xml \
        scene-assets/regulation.bin out/regulation.bin out/oath2-build.json

Paramdef: https://github.com/soulsmods/Paramdex (ER/Defs/SpEffect.xml). Running it on an already
patched regulation is a verified no-op.
