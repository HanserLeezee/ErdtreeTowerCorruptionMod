Based on hudhook 72e1cc062c35df847b703e5507b82eb21d80d82c (MIT).

Late DX12 attachment discovers a direct command queue by observing a PRESENT
transition of an actual swap-chain back buffer and submission of that command
list on the swap chain's device. It no longer probes private swap-chain memory.
Factory and ResizeBuffers1 capture remain the preferred initialization paths.
Evidence is bounded, consumed on submission, and invalidated on command-list
reset, buffer resize, pipeline reset, and unhook. Back buffers are not retained.

IME text input (renderer/ime.rs, renderer/pipeline.rs, renderer/input.rs, lib.rs):
the game window has no IME context, and IME text reached imgui only through
WM_CHAR produced by the default window procedure, which the game's procedure
does not forward. While imgui wants text, the window procedure associates the
default IME context (restoring the original afterwards), reads committed text
from WM_IME_COMPOSITION on the window thread, feeds it to imgui as characters
and strips the result flags so it is not repeated through WM_IME_CHAR/WM_CHAR.
WM_IME_CHAR alone (some IMEs) is decoded with surrogate pairs. The IME windows
follow the active field (`set_ime_position`). WM_CHAR no longer panics on a
lone surrogate. Needs the windows crate feature Win32_UI_Input_Ime.
