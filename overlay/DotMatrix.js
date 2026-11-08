.pragma library

var capacities = [1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 6, 5, 5, 4, 4, 3, 3, 2, 2, 1, 1, 1];

function bufferQuantum(ratio) {
    // A fractional logical Canvas size can itself be rounded before allocating
    // its buffer. Make BOTH logical and physical box dimensions whole pixels.
    // Wayland fractional scales are rational values with denominator <= 120.
    for (var denominator = 1; denominator <= 120; denominator++) {
        var numerator = Math.round(ratio * denominator);
        if (Math.abs(numerator / denominator - ratio) < 0.000001) return numerator;
    }
    return 1;
}

function gridLayout(scale, border, quantum) {
    // Design grid FIRST, in actual window-buffer pixels. Never squeeze or
    // stretch it into an independently sized box. Both axes use the same pitch.
    var side = Math.max(1, Math.round(2 * scale));
    var gap = Math.max(1, Math.round(scale));
    var pitch = side + gap;
    var inset = border + gap;
    var gridWidth = 27 * side + 26 * gap;
    var gridHeight = 6 * side + 5 * gap;
    var alignment = Math.max(1, quantum || 1);
    var width = Math.ceil((gridWidth + 2 * inset) / alignment) * alignment;
    var height = Math.ceil((gridHeight + 2 * inset) / alignment) * alignment;
    return {
        valid: true,
        side: side,
        gap: gap,
        x: Math.floor((width - gridWidth) / 2),
        y: Math.floor((height - gridHeight) / 2),
        stepX: pitch,
        stepY: pitch,
        width: width,
        height: height
    };
}

function snap(value, ratio) {
    return Math.round(value * ratio) / ratio;
}

function cells(grid, counts, processing, phase) {
    var result = [];
    if (!grid.valid) return result;
    for (var column = 0; column < 27; column++) {
        for (var row = 0; row < 6; row++) {
            var opacity = dotOpacity(column, row, counts[column]);
            if (processing && opacity > 0.2) opacity = Math.min(opacity, processingOpacity(column, phase));
            result.push({column: column, row: row, x: grid.x + column * grid.stepX,
                y: grid.y + row * grid.stepY, width: grid.side, height: grid.side, opacity: opacity});
        }
    }
    return result;
}

function clamp(value, low, high) {
    return Math.max(low, Math.min(high, value));
}

function startRow(count) {
    return Math.round((6 - count) / 2);
}

function horizontalOpacity(column) {
    var distance = Math.abs(column - 13);
    return distance <= 2 ? 1 : distance <= 5 ? 0.8 : distance <= 8 ? 0.6 : 0.4;
}

function dotOpacity(column, row, count) {
    var start = startRow(count);
    if (row < start || row >= start + count) return 0.2;
    var distance = Math.abs(row - (start + (count - 1) / 2));
    var vertical = distance <= 0.5 ? 1 : distance <= 1.5 ? 0.8 : distance <= 2.5 ? 0.6 : 0.4;
    return Math.min(horizontalOpacity(column), vertical);
}

// Deterministic noise makes demo captures/tests reproducible. No microphone use.
function noise(seed) {
    var value = Math.sin(seed * 127.1 + 311.7) * 43758.5453;
    return (value - Math.floor(value)) * 2 - 1;
}

function speechEnvelope(time) {
    var phrase = Math.max(0, Math.sin(time * 1.7));
    var syllables = 0.55 + 0.25 * Math.sin(time * 11.3) + 0.2 * Math.sin(time * 19.1);
    return clamp(phrase * syllables * 1.4 + noise(Math.floor(time * 12.5)) * 0.06, 0, 1);
}

function activeCounts(amplitude, time) {
    var a = isFinite(amplitude) ? clamp(amplitude, 0, 1) : 0;
    var result = [];
    var previous = 1;
    // Mirror both sides, with a monotone ascent to the center: fluctuations
    // cannot break the symmetric hill or exceed the supplied silhouette.
    for (var column = 0; column <= 13; column++) {
        var weight = 0.65 + 0.35 * (1 - Math.abs(column - 13) / 13);
        var jitter = noise(column + Math.floor(time * 12.5) * 17) * 0.45 * a;
        var count = clamp(Math.round(1 + a * (capacities[column] - 1) * weight + jitter), previous, capacities[column]);
        result[column] = count;
        result[26 - column] = count;
        previous = count;
    }
    return result;
}

function processingOpacity(column, time) {
    var head = (Math.floor(time * 12.5) % 31) - 2;
    var distance = Math.abs(column - head);
    return distance <= 1 ? 1 : distance <= 2 ? 0.8 : distance <= 3 ? 0.6 : 0.4;
}
