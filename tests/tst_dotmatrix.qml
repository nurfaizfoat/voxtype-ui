import QtQuick
import QtTest
import "../overlay/DotMatrix.js" as Matrix

TestCase {
    name: "DotMatrix"

    function test_pixelSnappedLayout() {
        for (var scale of [1, 1.3, 1.56, 1.95, 2, 2.6, 3, 3.9]) {
            for (var border of [0, 1, 2, 3, 4]) {
                var layout = Matrix.gridLayout(scale, border);
                var width = layout.width;
                var height = layout.height;
                verify(layout.valid);
                compare(layout.side, Math.round(layout.side));
                compare(layout.stepX, Math.round(layout.stepX), "Uniform physical-pixel column pitch.");
                compare(layout.stepY, Math.round(layout.stepY), "Uniform physical-pixel row pitch.");
                compare(layout.stepX, layout.stepY, "Identical pitch on both axes.");
                compare(layout.stepX - layout.side, layout.gap);
                compare(layout.x - border, layout.gap, "Dots never touch the frame.");
                for (var c = 0; c < 27; c++) {
                    var x = layout.x + Math.round(c * layout.stepX);
                    verify(x >= border && x + layout.side <= width - border);
                    if (c > 0) verify(x - (layout.x + Math.round((c - 1) * layout.stepX)) >= layout.side + 1);
                }
                for (var r = 0; r < 6; r++) {
                    var y = layout.y + Math.round(r * layout.stepY);
                    verify(y >= border && y + layout.side <= height - border);
                    if (r > 0) verify(y - (layout.y + Math.round((r - 1) * layout.stepY)) >= layout.side + 1);
                }
            }
        }
    }

    function test_fractionalDisplayLayoutAndOrigin() {
        for (var ratio of [1, 1.25, 1.5, 1.75, 2]) {
            var border = Math.round(2 * ratio);
            var layout = Matrix.gridLayout(1.56 * ratio, border, Matrix.bufferQuantum(ratio));
            var width = layout.width;
            var height = layout.height;
            verify(Math.abs(width / ratio - Math.round(width / ratio)) < 0.00001);
            verify(Math.abs(height / ratio - Math.round(height / ratio)) < 0.00001);
            verify(layout.valid);
            verify(layout.side >= 3, "Larger UI must retain clearly visible dots.");
            compare(layout.side, Math.round(layout.side));
            verify(layout.x >= border && layout.y >= border);
            verify(layout.x + 26 * layout.stepX + layout.side <= width - border);
            verify(layout.y + 5 * layout.stepY + layout.side <= height - border);
            for (var origin of [0, 12, 34, 12.5, -5.3]) {
                var physical = Matrix.snap(origin, ratio) * ratio;
                verify(Math.abs(physical - Math.round(physical)) < 0.00001);
            }
        }
    }

    function test_referenceShape() {
        compare(Matrix.capacities.length, 27);
        var histogram = { "20": 0, "40": 0, "60": 0, "80": 0, "100": 0 };
        for (var c = 0; c < 27; c++) {
            compare(Matrix.capacities[c], Matrix.capacities[26 - c]);
            for (var r = 0; r < 6; r++)
                histogram[Math.round(Matrix.dotOpacity(c, r, Matrix.capacities[c]) * 100)]++;
        }
        compare(histogram["20"], 70);
        compare(histogram["40"], 14);
        compare(histogram["60"], 38);
        compare(histogram["80"], 30);
        compare(histogram["100"], 10);
        compare(Matrix.startRow(6), 0);
        compare(Matrix.startRow(4), 1);
        compare(Matrix.startRow(2), 2);
        compare(Matrix.startRow(1), 3);
    }

    function test_speechIsBoundedSymmetricAndStepped() {
        var signatures = {};
        for (var tick = 0; tick < 250; tick++) {
            var time = tick * 0.08;
            var amplitude = Matrix.speechEnvelope(time);
            verify(amplitude >= 0 && amplitude <= 1);
            var counts = Matrix.activeCounts(amplitude, time);
            signatures[counts.join(",")] = true;
            for (var c = 0; c < 27; c++) {
                verify(counts[c] >= 1 && counts[c] <= Matrix.capacities[c]);
                compare(counts[c], counts[26 - c]);
                if (c < 13) verify(counts[c] <= counts[c + 1]);
                for (var r = 0; r < 6; r++) {
                    var opacity = Matrix.dotOpacity(c, r, counts[c]);
                    verify([0.2, 0.4, 0.6, 0.8, 1].indexOf(opacity) >= 0);
                    var processing = Math.min(opacity, Matrix.processingOpacity(c, time));
                    verify([0.2, 0.4, 0.6, 0.8, 1].indexOf(processing) >= 0);
                }
            }
        }
        verify(Object.keys(signatures).length > 10, "Speech must visibly vary.");
    }

    function test_silenceAndInvalidAudio() {
        for (var amplitude of [0, -1, NaN, Infinity])
            compare(Matrix.activeCounts(amplitude, 1), Array(27).fill(1));
        for (var count of Matrix.activeCounts(2, 1)) verify(count >= 1 && count <= 6);
    }
}
