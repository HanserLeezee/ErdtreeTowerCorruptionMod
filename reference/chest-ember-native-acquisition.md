# Chest ember native acquisition research

## Scope and status

The user requires the real game acquisition window with the existing ember artwork, not an overlay imitation. `ember_notice.rs` now uses the acquisition queue, replacing the uninstalled text-popup prototype.

The display implementation and artwork integration have been built and checked offline. In-game presentation is not yet verified. The code calls the signature-checked queue append function, never the inventory grant function. Resources add goods 9900000..9900025 and native icons 65000..65025; all 26 existing RGBA artworks are packed into a BGRA8 DDS atlas for both quality variants. Goods have no use effect and are not inserted into inventory.

`SceneBuilder <game> <output> <scene-assets> ember-pickup-patch` generates the six resources. Run from the repository root; `TOWER_PARAMDEFS` optionally selects the PARAMDEF directory. Original goods, texture bytes and layout bytes are checked for preservation, and generated files are read back.

Three native notice tests and six existing chest tests pass. Debug menu > Build > native ember pickup preview queues SpiritTwin and closes the debug overlay; it does not issue a debug gameplay command or save a changed run. The installed debug key should be read from `tower-options.ini` (currently Delete).

## Evidence (2026-11-03)

Executable: `F:/SteamLibrary/steamapps/common/ELDEN RING/Game/eldenring.exe`.
SHA-256: `f9118fea2cfd8fa3901e8060bfadb1552cb2471771ded8a5b662c4e298ffb328`.
All addresses below are RVAs specific to this executable; they are research observations, not a supported ABI.

- Existing `native/src/items.rs` calls item grant at `0x5618b0`.
- After successful inventory insertion, calls at `0x561a29` and `0x561b86` append an acquisition record through `0x5653f0`, passing `MapItemMan + 0xa0`.
- Append copies item ID, quantity, gem ID, and three flag bytes into a 16-byte record. It does not itself grant inventory. Capacity is 10; the count is at `MapItemMan + 0x148`.
- Consumer at `0x564b4e` iterates these records, calls `0x77aeb0`, then removes processed entries. This is the promising display-only route, but downstream side effects and flag meanings still require validation.
- Consumer obtains its manager from singleton pointer RVA `0x3d73440`. Initialization at `0x76be62` calls constructor `0x77ac50`, storing the result at `0x76be68`.
- Executable contains `CSItemGetMenuMan` at `0x2aa3290` and RTTI name `.?AVCSItemGetMenuManImpl@CS@@` at `0x3c99e20`. Mapping this RTTI to the observed manager must still be checked before treating the signature as established.
- `0x77aeb0` constructs item presentation data, queues UI entries and invokes additional helpers. A direct call or queue insertion must not be deployed until these helpers and prerequisites are understood.

## Artwork and text

- `native/src/reward_ui.rs::ember_icon` resolves 26 overlay artworks to IDs `990000..990025`; installed files are all 160x160 RGBA (102400 bytes).
- These are overlay files, not registered native item icons.
- Native goods/accessory `iconId` is `u16`. Reusing the overlay IDs as native IDs would overflow.
- `scene-builder/RewardAssets.cs` and `reward_icons.py` document the existing native atlas structure: `01_common.tpf.dcx`, `01_common.sblytbnd.dcx`, `SB_Icon*`, and `MENU_ItemIcon_<id>.png` layout entries.
- Integration uses collision-checked native icon IDs, atlas/layout entries, and display-only item definitions with matching Chinese FMG name entries. Existing item names/icons are not replaced; display-only items are not granted.
- Preserve both UI resource variants required by the active loader; do not assume overlay RGBA files can be consumed by Scaleform.

## Rejected shortcuts / next validation

- Local ESD documentation lists `OpenItemAcquisitionMenu` (bank 1, command 65), but its `games_used` list excludes ER. It is not proof of a callable ER command.
- Grok Search returned HTTP 401; no external implementation was verified.
- Do not call inventory grant then remove a fake item: that introduces inventory/save and multiplayer side effects.
- Next: establish manager RTTI/signature and display helper side effects, prepare isolated native icon/text/param resources, then perform a one-shot diagnostic on a backed-up test save. Verify original inventory quantities are unchanged and that the real icon/name window opens and dismisses normally. Only then connect it to the post-save chest claim path with version checks, queue limits, and once-only notification handling.
