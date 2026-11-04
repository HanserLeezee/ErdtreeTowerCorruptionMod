using System;
using System.Security.Cryptography;
using System.Text;

namespace Tower.Launcher
{
    internal static class LobbyLaunch
    {
        public static string? BlockReason(bool game, bool launching, bool worker, bool updating)
        {
            if (game) return "游戏正在运行，联机密码已锁定；请保存退出后再开房、加入或改密码";
            if (launching || worker) return "游戏正在启动，暂时不能切换房间或修改联机密码，请稍候";
            if (updating) return "正在更新，完成后再开始游戏或修改联机密码";
            return null;
        }

        public static string Fingerprint(string password)
        {
            using var hash = SHA256.Create();
            return BitConverter.ToString(hash.ComputeHash(Encoding.UTF8.GetBytes(password))).Replace("-", "").ToLowerInvariant();
        }
    }
}
