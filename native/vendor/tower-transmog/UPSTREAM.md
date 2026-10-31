# Integrated armor appearance engine

Based on ThomasJClark/elden-ring-transmog, commit d3cdf379d71aac18b2691d05dffa0b61092b67bd (2.4.14), MIT; see LICENSE.txt.

Vendored dependencies: elden-x 363961dac2858b6bad9ced3900d2b1b0026aee0f, spdlog v1.13.0, Pattern16 728eac543cee7e7bf6fda445d7db06e3dc8a61d0, MinHook f5485b8454544c2f034c78f8f127c1d03dea3636. Their licenses are retained beside their source.

Tower integration replaces dummy inventory items and grace-menu patches with the archive appearance page and character-specific .appearance records. It exposes init/set entry points to the tower runtime. Posture/hurtbox, Mimic Tear and blood-message copy hooks are not installed. F8 stays reserved for the tower debugger. Network messages are bounded and checked against session members and armor categories.

The engine is automatically loaded as tower_transmog.dll, built and distributed with the main DLL. Players do not install or configure a second mod. Steamworks 1.59 headers/import library are build-only dependencies; no Steam DLL is redistributed.
