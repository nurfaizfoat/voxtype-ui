// Hyprland client coordinates and layer surfaces both use logical pixels.
// Applying the monitor's scale a second time would misalign fractional displays.
function windowRect(client, originX, originY, screenWidth, screenHeight) {
    if (!client || !client.address || client.class === "org.omarchy.screensaver"
            || client.mapped === false
            || client.hidden === true || client.visible === false
            || !client.at || !client.size)
        return null;

    var x = client.at[0], y = client.at[1];
    var width = client.size[0], height = client.size[1];
    var numbers = [x, y, width, height, originX, originY, screenWidth, screenHeight];
    if (!numbers.every(function(n) { return typeof n === "number" && isFinite(n); })
            || width <= 0 || height <= 0 || screenWidth <= 0 || screenHeight <= 0)
        return null;

    x -= originX;
    y -= originY;
    if (x >= screenWidth || y >= screenHeight || x + width <= 0 || y + height <= 0)
        return null;

    // Keep the original rectangle; the surface clips it. Clamping the rectangle
    // would paint a false border where a spanning window crosses a monitor edge.
    return { x: x, y: y, width: width, height: height };
}

function normalizedAddress(address) {
    var value = typeof address === "string" ? address.trim().toLowerCase().replace(/^0x/, "") : "";
    return /^[0-9a-f]+$/.test(value) && !/^0+$/.test(value) ? value : "";
}

function energy(peak, rms) {
    if (typeof peak !== "number" || !isFinite(peak)
            || typeof rms !== "number" || !isFinite(rms))
        return 0;
    var linear = Math.max(0, rms) * 5 + Math.max(0, peak) * 0.8;
    // Display-only companding: quiet speech should use more than one or two of
    // the six rows. A small dead zone leaves near-zero bridge noise at rest.
    // Never change microphone gain, VAD, or the audio sent to transcription.
    var floor = 0.012;
    var full = 0.40;
    var normalized = Math.min(1, Math.max(0, (linear - floor) / (full - floor)));
    return Math.pow(normalized, 0.55);
}

function smoothLevel(previous, target, elapsedMs) {
    var old = typeof previous === "number" && isFinite(previous)
        ? Math.min(1, Math.max(0, previous)) : 0;
    var next = typeof target === "number" && isFinite(target)
        ? Math.min(1, Math.max(0, target)) : 0;
    var dt = typeof elapsedMs === "number" && isFinite(elapsedMs)
        ? Math.min(250, Math.max(0, elapsedMs)) : 0;
    // Reach 86% of an onset in one 40ms frame; release over about 140ms rather
    // than flickering between syllables. Timing is independent of tick length.
    var amount = 1 - Math.exp(-dt / (next > old ? 20 : 140));
    var level = old + (next - old) * amount;
    return next === 0 && level < 0.015 ? 0 : level;
}

function appendSample(samples, value, limit) {
    var next = samples.slice(Math.max(0, samples.length - limit + 1));
    next.push(Math.min(1, Math.max(0, value)));
    return next;
}

function accentFromToml(text, fallback) {
    var match = /^\s*accent\s*=\s*["'](#[0-9a-fA-F]{6})["']\s*(?:#.*)?$/m.exec(text);
    return match ? match[1] : fallback;
}
