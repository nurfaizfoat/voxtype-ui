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
    return Math.min(1, Math.max(0, Math.max(0, rms) * 5 + Math.max(0, peak) * 0.8));
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
