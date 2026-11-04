using System;
using System.IO;
using System.Runtime.InteropServices;

namespace Tower.Assets
{
    internal static class OodleDcx
    {
        [DllImport("kernel32", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
        [DllImport("oo2core_6_win64.dll")] static extern long OodleLZ_GetDecodeBufferSize(long size, bool corruptionPossible);
        [DllImport("oo2core_6_win64.dll")] static extern long OodleLZ_GetCompressedBufferSizeNeeded(long size);
        [DllImport("oo2core_6_win64.dll")] static extern IntPtr OodleLZ_CompressOptions_GetDefault(int compressor, int level);
        [DllImport("oo2core_6_win64.dll")] static extern long OodleLZ_Decompress(byte[] input, long inputLength, byte[] output, long outputLength,
            int safe, int crc, int verbosity, IntPtr baseBuffer, long bufferSize, IntPtr callback, IntPtr user, IntPtr memory, long memorySize, int phase);
        [DllImport("oo2core_6_win64.dll")] static extern long OodleLZ_Compress(int compressor, byte[] input, long size, byte[] output,
            int level, IntPtr options, IntPtr dictionary, IntPtr matcher, IntPtr scratch, long scratchSize);
        public static void Load(string game, string hash)
        {
            var path = Path.Combine(game, "oo2core_6_win64.dll");
            if (AssetHash.FileHash(path) != hash) throw new InvalidDataException("游戏压缩库版本不匹配，请使用完整整合包。");
            if (LoadLibraryEx(path, IntPtr.Zero, 0x100 | 0x800) == IntPtr.Zero) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        }
        static int Big(byte[] bytes, int offset) { return checked((int)((uint)bytes[offset] << 24 | (uint)bytes[offset + 1] << 16 | (uint)bytes[offset + 2] << 8 | bytes[offset + 3])); }
        static void Validate(byte[] bytes)
        {
            if (bytes.Length < 76 || System.Text.Encoding.ASCII.GetString(bytes, 0, 4) != "DCX\0" ||
                System.Text.Encoding.ASCII.GetString(bytes, 40, 4) != "KRAK" || Big(bytes, 20) != 76)
                throw new InvalidDataException("Unsupported DCX format");
        }
        public static byte[] Decode(byte[] dcx)
        {
            Validate(dcx);
            int length = Big(dcx, 28), compressed = Big(dcx, 32);
            if (length <= 0 || length > 1024 * 1024 * 1024 || compressed <= 0 || compressed > dcx.Length - 76) throw new InvalidDataException("Invalid DCX length");
            var input = new byte[compressed]; Buffer.BlockCopy(dcx, 76, input, 0, compressed);
            long capacity = OodleLZ_GetDecodeBufferSize(length, true);
            if (capacity < length || capacity > length + 1024L * 1024) throw new InvalidDataException("Invalid decoder capacity");
            var output = new byte[capacity];
            if (OodleLZ_Decompress(input, input.Length, output, length, 1, 0, 0, IntPtr.Zero, 0, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, 0, 3) != length)
                throw new InvalidDataException("DCX decode failed");
            Array.Resize(ref output, length); return output;
        }
        public static byte[] Encode(byte[] input, byte[] header)
        {
            Validate(header);
            if (header.Length != 76 || Big(header, 28) != input.Length) throw new InvalidDataException("Invalid target DCX header");
            int level = header[48];
            if (level != 6) throw new InvalidDataException("Unsupported compression level");
            var options = Marshal.AllocHGlobal(48);
            try
            {
                // Oodle 2.6 options layout used by SoulsFormats; independent 256 KiB seek chunks.
                var defaults = new byte[48]; Marshal.Copy(OodleLZ_CompressOptions_GetDefault(8, level), defaults, 0, 48);
                Marshal.Copy(defaults, 0, options, 48); Marshal.WriteInt32(options, 8, 1); Marshal.WriteInt32(options, 12, 0x40000);
                long capacity = OodleLZ_GetCompressedBufferSizeNeeded(input.Length);
                if (capacity < 0 || capacity > input.LongLength * 2 + 1024 * 1024) throw new InvalidDataException("Invalid encoder capacity");
                var compressed = new byte[capacity];
                long size = OodleLZ_Compress(8, input, input.Length, compressed, level, options, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, 0);
                if (size <= 0 || size != Big(header, 32)) throw new InvalidDataException("重建压缩结果不匹配，请使用完整整合包。");
                var output = new byte[(76 + size + 15) / 16 * 16];
                Buffer.BlockCopy(header, 0, output, 0, 76); Buffer.BlockCopy(compressed, 0, output, 76, checked((int)size));
                return output;
            }
            finally { Marshal.FreeHGlobal(options); }
        }
    }
}
