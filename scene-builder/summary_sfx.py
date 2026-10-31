"""结算页音效：纯 Python 合成（不依赖外部素材），写出 scene-assets/sfx/*.wav（44.1 kHz、16 位、单声道）。

用法：python scene-builder/summary_sfx.py <仓库根目录>
风格：干净、偏工业感的界面音（参考手游活动结算）。每次运行结果相同（噪声用固定种子）。
- open      打开结算页：低频一顿 + 上扫的气流声
- tick1..3  分数滚动的“嗒”声（三个音高轮流用，免得机械）
- stamp     评级落下：低频重击 + 金属余响
- record    新纪录：上行的四个钟音 + 高频闪光
- hover     按钮悬停：很轻的嗒声
- confirm   按下按钮：两声短促的咔嗒
- lock1..6  分数每一位停下：带一点分量的咔嗒，音高一位比一位高
- sweep     高光扫过大数字：很轻的高频闪光
- total     总分落定：低频一顿 + 明亮的一声
- close     离开结算页：往回收的气流声
"""
import math, random, struct, sys, wave
from pathlib import Path

RATE = 44100
rng = random.Random(20261031)

def silence(seconds): return [0.0] * int(RATE * seconds)
def mix(dst, src, at=0.0, gain=1.0):
    start = int(at * RATE)
    if len(dst) < start + len(src): dst.extend([0.0] * (start + len(src) - len(dst)))
    for i, v in enumerate(src): dst[start + i] += v * gain
    return dst
def env_exp(n, tau, attack=0.002):
    a = max(1, int(attack * RATE))
    return [min(1.0, i / a) * math.exp(-(i / RATE) / tau) for i in range(n)]
def tone(freq, seconds, tau, attack=0.002, glide=None):
    """正弦；glide=(终点频率, 时间常数) 时频率指数滑向终点"""
    n = int(seconds * RATE); e = env_exp(n, tau, attack); out = []; phase = 0.0
    for i in range(n):
        f = freq if glide is None else glide[0] + (freq - glide[0]) * math.exp(-(i / RATE) / glide[1])
        phase += 2 * math.pi * f / RATE; out.append(math.sin(phase) * e[i])
    return out
def noise(seconds, tau, attack=0.002, cutoff=(4000, 4000), highpass=0.0):
    """一阶低通的噪声，截止频率从 cutoff[0] 线性移到 cutoff[1]；highpass>0 时再减去一份更低的低通"""
    n = int(seconds * RATE); e = env_exp(n, tau, attack); out = []; lp = 0.0; lp2 = 0.0
    for i in range(n):
        fc = cutoff[0] + (cutoff[1] - cutoff[0]) * i / max(1, n - 1)
        a = 1 - math.exp(-2 * math.pi * fc / RATE)
        lp += a * (rng.uniform(-1, 1) - lp)
        v = lp
        if highpass:
            b = 1 - math.exp(-2 * math.pi * highpass / RATE); lp2 += b * (lp - lp2); v = lp - lp2
        out.append(v * e[i])
    return out
def bell(freq, seconds, tau):
    """钟音：基频加两个非整数倍的泛音，泛音衰减得更快"""
    out = tone(freq, seconds, tau, 0.001)
    mix(out, tone(freq * 2.76, seconds, tau * 0.45, 0.001), gain=0.35)
    mix(out, tone(freq * 5.40, seconds, tau * 0.25, 0.001), gain=0.18)
    return out
def echo(sig, delay, feedback, count):
    out = list(sig)
    for k in range(1, count + 1): mix(out, sig, delay * k, feedback ** k)
    return out
def normalize(sig, peak_db):
    peak = max(1e-9, max(abs(v) for v in sig)); g = 10 ** (peak_db / 20) / peak
    fade = int(0.004 * RATE)
    out = [v * g for v in sig]
    for i in range(min(fade, len(out))): out[-1 - i] *= i / fade
    return out
def write(path, sig):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", max(-32767, min(32767, int(v * 32767)))) for v in sig))

def make_open():
    s = silence(0.6)
    mix(s, tone(70, 0.35, 0.09, 0.004, glide=(42, 0.12)), gain=0.9)
    mix(s, noise(0.5, 0.16, 0.06, cutoff=(500, 5200), highpass=180), 0.02, 0.55)
    mix(s, tone(1760, 0.25, 0.08, 0.02), 0.08, 0.05)
    return normalize(s, -6)
def make_tick(freq):
    s = tone(freq, 0.04, 0.006, 0.0005)
    mix(s, noise(0.01, 0.002, 0.0002, cutoff=(9000, 9000), highpass=2500), gain=0.5)
    return normalize(s, -10)
def make_stamp():
    s = silence(1.0)
    mix(s, tone(120, 0.7, 0.22, 0.001, glide=(44, 0.05)), gain=1.0)
    mix(s, noise(0.12, 0.03, 0.0005, cutoff=(2600, 400)), gain=0.9)
    ring = silence(0.9)
    for f, g in [(523, 0.5), (1312, 0.32), (2214, 0.2), (3330, 0.1)]: mix(ring, tone(f, 0.9, 0.28, 0.001), gain=g)
    mix(s, echo(ring, 0.071, 0.35, 4), 0.004, 0.32)
    return normalize(s, -3)
def make_record():
    s = silence(1.5)
    for k, f in enumerate([659.3, 830.6, 987.8, 1318.5]): mix(s, bell(f, 1.1, 0.42), 0.075 * k, 0.55 + 0.1 * k)
    mix(s, noise(0.9, 0.3, 0.12, cutoff=(9000, 12000), highpass=6000), 0.22, 0.25)
    return normalize(echo(s, 0.11, 0.25, 3), -5)
def make_hover():
    s = tone(1500, 0.03, 0.005, 0.0005)
    mix(s, noise(0.008, 0.002, 0.0002, cutoff=(7000, 7000), highpass=2000), gain=0.3)
    return normalize(s, -18)
def make_confirm():
    s = silence(0.22)
    for at, f in [(0.0, 880), (0.045, 1320)]:
        mix(s, tone(f, 0.12, 0.022, 0.0005), at, 1.0)
        mix(s, noise(0.012, 0.003, 0.0002, cutoff=(8000, 8000), highpass=2000), at, 0.5)
    mix(s, tone(110, 0.15, 0.04, 0.001), 0.0, 0.6)
    return normalize(s, -7)

def make_lock(step):
    f = 1150 * (2 ** (step / 7))
    s = tone(f, 0.09, 0.016, 0.0005)
    mix(s, tone(f * 2.01, 0.05, 0.006, 0.0005), gain=0.25)
    mix(s, tone(210, 0.07, 0.018, 0.001), gain=0.7)
    mix(s, noise(0.012, 0.003, 0.0002, cutoff=(9000, 9000), highpass=2500), gain=0.6)
    return normalize(s, -9)
def make_sweep():
    s = noise(0.55, 0.22, 0.12, cutoff=(6000, 13000), highpass=5000)
    mix(s, tone(2637, 0.5, 0.18, 0.08), 0.05, 0.08)
    mix(s, tone(3520, 0.45, 0.15, 0.08), 0.12, 0.06)
    return normalize(s, -16)
def make_total():
    s = silence(0.7)
    mix(s, tone(90, 0.4, 0.1, 0.001, glide=(50, 0.06)), gain=0.9)
    mix(s, noise(0.05, 0.012, 0.0005, cutoff=(3000, 600)), gain=0.5)
    mix(s, bell(1046.5, 0.6, 0.22), 0.01, 0.45)
    return normalize(s, -6)
def make_close():
    s = silence(0.5)
    mix(s, noise(0.4, 0.2, 0.15, cutoff=(5500, 400), highpass=150), 0.0, 0.7)
    mix(s, tone(60, 0.2, 0.06, 0.003, glide=(38, 0.08)), 0.28, 0.8)
    return normalize(s, -8)

out = Path(sys.argv[1]) / "scene-assets" / "sfx"; out.mkdir(parents=True, exist_ok=True)
sounds = {"open": make_open(), "tick1": make_tick(2350), "tick2": make_tick(2550), "tick3": make_tick(2750),
          "stamp": make_stamp(), "record": make_record(), "hover": make_hover(), "confirm": make_confirm(),
          "sweep": make_sweep(), "total": make_total(), "close": make_close()}
sounds.update({f"lock{k + 1}": make_lock(k) for k in range(6)})
for name, sig in sounds.items():
    write(out / f"{name}.wav", sig); print(f"{name}.wav {len(sig) / RATE:.2f} s")
