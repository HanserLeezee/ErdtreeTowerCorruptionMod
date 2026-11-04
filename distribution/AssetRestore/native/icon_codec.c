#define BCDEC_STATIC
#define BCDEC_IMPLEMENTATION
#include "bcdec.h"

__declspec(dllexport) int tower_decode_icon(const unsigned char* source, int source_size, unsigned char* target, int target_size) {
    int x, y;
    if (!source || !target || source_size != 25600 || target_size != 102400) return 0;
    for (y = 0; y < 40; y++) for (x = 0; x < 40; x++)
        bcdec_bc7(source + (y * 40 + x) * 16, target + (y * 4 * 160 + x * 4) * 4, 640);
    return 1;
}
