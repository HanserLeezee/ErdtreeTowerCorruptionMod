# Mausoleum encounter

The Dragonlord arena uses the Weeping Peninsula mausoleum (actor 1042330400), not the Deeproot rooftop-skull variant. The original 44 foot skulls, AI, descent effects, door ObjAct and attachment events are retained. Descent requires 12 destroyed skulls, relaxed from the native 45% threshold (20 of 44) at the user's request. The clone has no patrol route into the original overworld.

Set `TOWER_MAUSOLEUM_SOURCE` to an extracted vanilla resource directory containing:

- `m60_10_08_02.msb.dcx` (large-map placement)
- `m60_42_33_00.emevd.dcx` (events 200 and 250)
- `common_func.emevd.dcx`
- `m12_03_00_00.msb.dcx` (same-model dungeon drawing and tile-load templates; gameplay still uses Weeping Peninsula)

Run `SceneBuilder <game> <staging-output> <scene-assets> mausoleum-flow-patch`. `mausoleum-patch` is an alias for the same implementation. Copy the generated map, event and `arenas-resources.txt` back to `scene-assets`, then rebuild the native DLL because it embeds the resource fingerprints. `mausoleum-flow-verify` audits the staged map and events.

Private IDs: enabled 45011859, actor 45011860, awake 45011861, skull group 45011862, setup 45011863, lowered 45011864, opened 45011865, assets 45011700–45011778, specialized helpers 45011600–45011649. Common-event bytecode is parameter-bound into local helpers; the global common event file is unchanged. Original overworld flags are not written.

Native save data tracks descent and door opening independently of the one-time weapon receipt. The NPC offer does not complete either stage. The copy menu and reward request are blocked until both stages complete. Reload restores completed stages, while a fresh encounter starts standing with the door locked. Debug logs include `MAUSOLEUM_TRACE` and `MAUSOLEUM_FLOW`.

Automated checks cover resource references, parameter dependencies, door binding, native skull threshold, unrelated-map preservation, repeat generation, save roundtrips and copy gating. They do not simulate attacks, animation playback or player traversal in the running game.

The live port stalled at the aggregate asset-group ratio condition. Each of the 44 skull assets now has its own local destruction watcher/receipt (45011500–45011543). The descent helper waits for 12 receipts, then applies the original 12450 trigger. Receipts reset on encounter initialization. The room status and debug trace expose the actual count; this distinguishes missing asset-hit detection from an AI/animation stall after the threshold.
