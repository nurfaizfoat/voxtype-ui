"""Check ONLY a synthetic opaque test indicator after real Wayland composition."""
import json
import math
import os
from pathlib import Path
import selectors
import signal
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
MARKER = "VOXTYPE_PIXEL_PROBE "
CAPACITIES = [1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 5, 5, 4, 4, 3, 3, 2, 2, 1, 1, 1]


def check_capture(info):
    ratio = info["windowRatio"]
    grid = info["grid"]
    assert grid["stepX"] == grid["stepY"], "Grid pitch differs between axes"
    assert grid["stepX"] - grid["side"] == grid["gap"], "Irregular dot spacing"
    width, height = grid["width"], grid["height"]
    assert info["width"] * ratio == width and info["height"] * ratio == height
    assert float(info["width"]).is_integer() and float(info["height"]).is_integer()
    monitors = json.loads(subprocess.check_output(["hyprctl", "-j", "monitors"]))
    monitor = next(m for m in monitors if m["name"] == info["screenName"])
    assert abs(monitor["scale"] - ratio) < 0.00001, "Window buffer would be rescaled by compositor"
    geometry = f'{monitor["x"] + info["x"]},{monitor["y"] + info["y"]} {int(info["width"])}x{int(info["height"])}'
    capture = ROOT / "build/compositor-dots.png"
    subprocess.run(["grim", "-g", geometry, str(capture)], check=True)
    header = capture.read_bytes()
    assert header[:8] == b"\x89PNG\r\n\x1a\n"
    assert struct.unpack(">II", header[16:24]) == (width, height), "Capture is not native resolution"
    pixels = subprocess.check_output(["magick", str(capture), "-depth", "8", "RGB:-"])
    assert len(pixels) == width * height * 3
    accent = tuple(int(info["accent"][i:i + 2], 16) for i in (1, 3, 5))
    background = (13, 13, 13)
    border = info["border"]
    expected = [background] * (width * height)
    for y in range(height):
        for x in range(width):
            if x < border or y < border or x >= width - border or y >= height - border:
                expected[y * width + x] = accent
    for c, count in enumerate(CAPACITIES):
        start = math.floor((6 - count) / 2 + 0.5)
        horizontal = 1 if abs(c - 13) <= 2 else 0.8 if abs(c - 13) <= 5 else 0.6 if abs(c - 13) <= 8 else 0.4
        for r in range(6):
            opacity = 0.2
            if start <= r < start + count:
                distance = abs(r - (start + (count - 1) / 2))
                vertical = 1 if distance <= 0.5 else 0.8 if distance <= 1.5 else 0.6 if distance <= 2.5 else 0.4
                opacity = min(horizontal, vertical)
            color = tuple(round(13 * (1 - opacity) + channel * opacity) for channel in accent)
            x = grid["x"] + c * grid["stepX"]
            y = grid["y"] + r * grid["stepY"]
            for dy in range(grid["side"]):
                for dx in range(grid["side"]):
                    expected[(y + dy) * width + x + dx] = color
    for index, color in enumerate(expected):
        actual = pixels[index * 3:index * 3 + 3]
        assert all(abs(a - b) <= 1 for a, b in zip(actual, color)), (
            f"Composited pixel {index % width},{index // width}: {tuple(actual)} != {color}"
        )
    print(f'PASS real compositor: screen DPR {info["screenRatio"]}, window DPR {ratio}; '
          f'162 solid {grid["side"]}×{grid["side"]} squares, {grid["gap"]}px gaps on BOTH axes; '
          f'{info["width"]:g}×{info["height"]:g} logical box')


def main():
    (ROOT / "build").mkdir(exist_ok=True)
    env = dict(os.environ, VOXTYPE_UI_CAPTURE_PROBE="1", QT_FORCE_STDERR_LOGGING="1")
    process = subprocess.Popen(["qs", "--no-color", "--path", str(ROOT / "overlay/PixelProbe.qml")],
                               env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    pending = b""
    deadline = time.monotonic() + 8
    try:
        while time.monotonic() < deadline:
            if not selector.select(max(0, deadline - time.monotonic())):
                break
            chunk = os.read(process.stdout.fileno(), 65536)
            if not chunk:
                break
            pending += chunk
            while b"\n" in pending:
                raw, pending = pending.split(b"\n", 1)
                line = raw.decode(errors="replace")
                if MARKER in line:
                    check_capture(json.loads(line.split(MARKER, 1)[1]))
                    return
        raise RuntimeError("Pixel probe did not become ready; run inside a Hyprland session")
    finally:
        selector.close()
        try:
            process.send_signal(signal.SIGINT)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


if __name__ == "__main__":
    main()
