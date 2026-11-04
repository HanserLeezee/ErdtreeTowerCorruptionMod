using System.IO.Compression;
using Tower.Assets;
using Xunit;

public class AssetPatchTests
{
    [Theory]
    [InlineData(-1, 640)] [InlineData(0, 639)] [InlineData(1, 640)] [InlineData(int.MaxValue, int.MaxValue)]
    public void InvalidIconRangesAreRejectedBeforeNativeDecode(int offset, int stride)
    {
        Assert.Throws<InvalidDataException>(() => IconCodec.Decode(new byte[25600], offset, stride));
    }

    [Fact]
    public void ShiftedAndChangedContentRoundTrips()
    {
        var source = new byte[32768]; new Random(14).NextBytes(source);
        var target = new byte[45000]; new Random(19).NextBytes(target);
        Array.Copy(source, 0, target, 731, source.Length);
        using var patch = new MemoryStream();
        AssetPatch.Create(source, target, patch);
        Assert.True(patch.Length < target.Length / 2);
        patch.Position = 0;
        Assert.Equal(target, AssetPatch.Apply(source, patch, target.Length));
    }

    [Theory]
    [InlineData(0)] [InlineData(10)] [InlineData(256)] [InlineData(257)]
    public void SmallAndRepeatedDataRoundTrips(int length)
    {
        var bytes = new byte[length];
        using var patch = new MemoryStream();
        AssetPatch.Create(bytes, bytes, patch); patch.Position = 0;
        Assert.Equal(bytes, AssetPatch.Apply(bytes, patch, length));
    }

    [Fact]
    public void RejectsCopyOutsideSourceAndWrongLength()
    {
        using var patch = new MemoryStream();
        using (var zip = new DeflateStream(patch, CompressionLevel.Optimal, true))
        using (var writer = new BinaryWriter(zip)) { writer.Write((byte)1); writer.Write(8); writer.Write(16); }
        patch.Position = 0;
        Assert.Throws<InvalidDataException>(() => AssetPatch.Apply(new byte[12], patch, 16));
        using var valid = new MemoryStream(); AssetPatch.Create(new byte[10], new byte[10], valid);
        valid.Position = 0;
        Assert.ThrowsAny<IOException>(() => AssetPatch.Apply(new byte[10], valid, 20));
    }
}
