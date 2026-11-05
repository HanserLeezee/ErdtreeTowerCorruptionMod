using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Tower.Assets
{
    public static class IconCodec
    {
        [DllImport("Tower.IconCodec.dll", CallingConvention = CallingConvention.Cdecl)]
        static extern int tower_decode_icon(byte[] source, int sourceSize, byte[] target, int targetSize);

        public static byte[] Decode(byte[] atlas, int offset, int stride)
        {
            if (offset < 0 || stride < 640 || (long)offset + 39L * stride + 640 > atlas.Length)
                throw new InvalidDataException("Invalid icon atlas range");
            var blocks = new byte[25600];
            for (int row = 0; row < 40; row++) Buffer.BlockCopy(atlas, offset + row * stride, blocks, row * 640, 640);
            var result = new byte[102400];
            if (tower_decode_icon(blocks, blocks.Length, result, result.Length) != 1) throw new InvalidDataException("Icon decode failed");
            return result;
        }
    }
}
