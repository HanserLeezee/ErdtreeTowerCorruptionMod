using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;

namespace Tower.Assets
{
    public static class AssetPatch
    {
        const int Block = 256;
        static ulong Hash(byte[] bytes, int offset)
        {
            ulong hash = 0;
            for (int i = 0; i < Block; i++) hash = unchecked(hash * 257 + bytes[offset + i]);
            return hash;
        }
        static bool Equal(byte[] a, int x, byte[] b, int y)
        {
            for (int i = 0; i < Block; i++) if (a[x + i] != b[y + i]) return false;
            return true;
        }
        public static void Create(byte[] source, byte[] target, Stream output)
        {
            ulong power = 1;
            for (int i = 1; i < Block; i++) power = unchecked(power * 257);
            var index = new Dictionary<ulong, int>();
            for (int i = 0; i <= source.Length - Block; i += Block)
            {
                var hash = Hash(source, i);
                if (!index.ContainsKey(hash)) index.Add(hash, i);
            }
            using (var zip = new DeflateStream(output, CompressionLevel.Optimal, true))
            using (var writer = new BinaryWriter(zip))
            {
                int pos = 0, start = 0;
                ulong hash = target.Length >= Block ? Hash(target, 0) : 0;
                while (pos <= target.Length - Block)
                {
                    int at;
                    if (index.TryGetValue(hash, out at) && Equal(source, at, target, pos))
                    {
                        if (pos > start) { writer.Write((byte)0); writer.Write(pos - start); writer.Write(target, start, pos - start); }
                        int size = Block;
                        while (at + size < source.Length && pos + size < target.Length && source[at + size] == target[pos + size]) size++;
                        writer.Write((byte)1); writer.Write(at); writer.Write(size);
                        pos += size; start = pos;
                        if (pos <= target.Length - Block) hash = Hash(target, pos);
                    }
                    else
                    {
                        if (pos < target.Length - Block) hash = unchecked((hash - target[pos] * power) * 257 + target[pos + Block]);
                        pos++;
                    }
                }
                if (start < target.Length) { writer.Write((byte)0); writer.Write(target.Length - start); writer.Write(target, start, target.Length - start); }
            }
        }
        public static byte[] Apply(byte[] source, Stream patch, int length)
        {
            if (length < 0 || length > 1024 * 1024 * 1024) throw new InvalidDataException("Invalid asset size");
            var target = new byte[length];
            using (var zip = new DeflateStream(patch, CompressionMode.Decompress, true))
            using (var reader = new BinaryReader(zip))
            {
                int pos = 0;
                while (pos < length)
                {
                    byte operation = reader.ReadByte();
                    int at = operation == 1 ? reader.ReadInt32() : 0;
                    int size = reader.ReadInt32();
                    if (size <= 0 || size > length - pos) throw new InvalidDataException("Invalid patch length");
                    if (operation == 1)
                    {
                        if (at < 0 || at > source.Length - size) throw new InvalidDataException("Invalid source range");
                        Buffer.BlockCopy(source, at, target, pos, size);
                    }
                    else if (operation == 0)
                    {
                        int end = pos + size, read = pos;
                        while (read < end)
                        {
                            int count = reader.Read(target, read, end - read);
                            if (count == 0) throw new EndOfStreamException();
                            read += count;
                        }
                    }
                    else throw new InvalidDataException("Invalid patch operation");
                    pos += size;
                }
                if (reader.BaseStream.ReadByte() != -1) throw new InvalidDataException("Trailing patch data");
            }
            return target;
        }
    }
}
