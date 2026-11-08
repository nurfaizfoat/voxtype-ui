import QtQuick
import QtTest
import "../overlay"
import "../overlay/DotMatrix.js" as Matrix
import "../overlay/Geometry.js" as Levels

Item {
    id: scene
    width: 640
    height: 420

    Rectangle { anchors.fill: parent; color: "#11141b" }
    Rectangle {
        x: 40; y: 35; width: 560; height: 340
        color: "#1e232d"
        Text {
            anchors.centerIn: parent
            text: "Example document"
            color: "#b8becb"
            font.pixelSize: 18
        }
    }

    WindowIndicator {
        id: overlay
        x: 40; y: 35; width: 560; height: 340
        animate: false
    }

    DotIndicator {
        id: indicator
        x: 10; y: 10
        animate: false
    }

    TestCase {
        name: "OverlayRendering"
        when: windowShown

        function init() {
            indicator.animate = false;
            indicator.visible = true;
            indicator.simulate = true;
            indicator.daemonState = "recording";
            indicator.phase = 0;
            indicator.pixelScale = 1;
            indicator.sizeMultiplier = 1;
            indicator.borderThickness = 1;
            indicator.accent = "#267814";
            overlay.width = 560;
            overlay.height = 340;
            overlay.accent = "#FFD60A";
            overlay.borderThickness = 2;
        }

        function verifyPixel(image, x, y, opacity) {
            var background = 13;
            var foreground = [indicator.accent.r * 255, indicator.accent.g * 255, indicator.accent.b * 255];
            var actual = [image.red(x, y), image.green(x, y), image.blue(x, y)];
            for (var i = 0; i < 3; i++)
                verify(Math.abs(actual[i] - Math.round(background * (1 - opacity) + foreground[i] * opacity)) <= 1,
                    "Pixel " + x + "," + y + " channel " + i + " opacity " + opacity);
        }

        function test_enlargedThemedPixelsAndSquareDots() {
            indicator.sizeMultiplier = 1.56;
            indicator.borderThickness = 2;
            for (var accent of ["#FFD60A", "#89b4fa", "#f38ba8"]) {
                indicator.accent = accent;
                for (var scale of [1, 2, 3]) {
                    indicator.pixelScale = scale;
                    wait(30);
                    var image = grabImage(indicator);
                    var side = Math.round(2 * 1.56 * scale);
                    var gap = Math.round(1.56 * scale);
                    compare(image.width, 27 * side + 26 * gap + 2 * (2 * scale + gap));
                    compare(image.height, 6 * side + 5 * gap + 2 * (2 * scale + gap));
                    compare(indicator.grid.stepX, indicator.grid.stepY);
                    compare(indicator.border.width, 2 * scale, "Match border independently of UI size.");
                    var expected = Array(image.width * image.height).fill(0);
                    var thickness = indicator.border.width;
                    for (var y = 0; y < image.height; y++)
                        for (var x = 0; x < image.width; x++)
                            if (x < thickness || y < thickness || x >= image.width - thickness || y >= image.height - thickness)
                                expected[y * image.width + x] = 1;
                    var dots = 0;
                    for (var dot of indicator.cells) {
                        dots++;
                        compare(dot.width, dot.height, "Each dot must be exactly square.");
                        compare(dot.width, Math.round(dot.width));
                        compare(dot.x, Math.round(dot.x));
                        compare(dot.y, Math.round(dot.y));
                        verify(dot.x >= thickness && dot.y >= thickness);
                        verify(dot.x + dot.width <= image.width - thickness);
                        verify(dot.y + dot.height <= image.height - thickness);
                        for (var dy = 0; dy < dot.height; dy++)
                            for (var dx = 0; dx < dot.width; dx++)
                                expected[(dot.y + dy) * image.width + dot.x + dx] = dot.opacity;
                    }
                    compare(dots, 162);
                    for (var py = 0; py < image.height; py++)
                        for (var px = 0; px < image.width; px++)
                            verifyPixel(image, px, py, expected[py * image.width + px]);
                    if (accent === "#FFD60A") image.save("build/indicator-themed-" + scale + "x.png");
                }
            }
        }

        function test_borderAndAccentChangesPropagate() {
            var visual = overlay.children[0];
            compare(visual.width, 141);
            compare(visual.height, 36);
            for (var thickness of [0, 1, 2, 3, 4]) {
                overlay.borderThickness = thickness;
                overlay.accent = thickness % 2 ? "#89b4fa" : "#f38ba8";
                compare(visual.border.width, thickness);
                compare(visual.border.color, overlay.accent);
                verify(visual.grid.valid);
                compare(visual.grid.side, 3, "Never shrink dots to fit a thicker border.");
                compare(visual.width, 137 + 2 * thickness);
                compare(visual.height, 32 + 2 * thickness);
            }
        }

        function test_exactPixelsAtIntegerScales() {
            for (var scale of [1, 2, 3]) {
                indicator.pixelScale = scale;
                wait(30);
                var image = grabImage(indicator);
                compare(image.width, 84 * scale);
                compare(image.height, 21 * scale);
                for (var y = 0; y < image.height; y++) {
                    for (var x = 0; x < image.width; x++) {
                        var lx = Math.floor(x / scale);
                        var ly = Math.floor(y / scale);
                        var opacity = 0;
                        if (lx === 0 || lx === 83 || ly === 0 || ly === 20) opacity = 1;
                        else if (lx >= 2 && lx < 82 && ly >= 2 && ly < 19
                                 && (lx - 2) % 3 < 2 && (ly - 2) % 3 < 2) {
                            var c = Math.floor((lx - 2) / 3);
                            var r = Math.floor((ly - 2) / 3);
                            opacity = Matrix.dotOpacity(c, r, Matrix.capacities[c]);
                        }
                        verifyPixel(image, x, y, opacity);
                    }
                }
                image.save("build/indicator-reference-" + scale + "x.png");
            }
        }

        function test_statesRenderWithoutWindowOutline() {
            indicator.visible = false;
            for (var state of ["recording", "transcribing", "streaming"]) {
                overlay.daemonState = state;
                wait(50);
                var image = grabImage(overlay);
                compare(image.width, 560);
                compare(image.height, 340);
                compare(image.pixel(100, 100), "#1e232d", "Do not paint over document interior.");
                compare(image.pixel(1, 150), "#1e232d", "No window outline or glow.");
                compare(image.pixel(210, 292), "#ffd60a", "Grid-fitted indicator border at bottom center.");
                grabImage(scene).save("build/preview-" + state + ".png");
            }
        }

        function test_tinyWindowHidesRatherThanOverflows() {
            overlay.width = 60;
            overlay.height = 40;
            compare(overlay.children[0].visible, false);
            overlay.width = 141;
            overlay.height = 60;
            compare(overlay.children[0].visible, true);
            compare(overlay.children[0].x, 0);
        }

        function test_animationLifecycleAndLiveSilence() {
            indicator.simulate = false;
            indicator.level = 0;
            indicator.animate = true;
            compare(indicator.counts, Array(27).fill(1));
            tryVerify(function() { return indicator.phase > 0; }, 500);
            var phase = indicator.phase;
            indicator.visible = false;
            wait(180);
            compare(indicator.phase, phase, "Hidden indicators must not animate.");
            indicator.visible = true;
            tryVerify(function() { return indicator.phase > phase; }, 500);
            indicator.animate = false;
            phase = indicator.phase;
            wait(180);
            compare(indicator.phase, phase);
            compare(indicator.counts, Matrix.capacities);
        }

        function test_liveSensitivityAndUnchangedProcessingGeometry() {
            indicator.simulate = false;
            indicator.animate = true;
            indicator.sizeMultiplier = 1.56;
            indicator.pixelScale = 3;
            indicator.borderThickness = 2;
            indicator.accent = "#6e9cf7";
            compare(indicator.frameInterval, 40);
            var grid = JSON.stringify(indicator.grid);
            var width = indicator.width;
            var height = indicator.height;
            var cases = [
                {name: "silence", peak: 0, rms: 0, minimum: 1},
                {name: "quiet", peak: 0.04, rms: 0.008, minimum: 3},
                {name: "moderate", peak: 0.12, rms: 0.025, minimum: 4},
                {name: "strong", peak: 0.3, rms: 0.075, minimum: 6}
            ];
            for (var sample of cases) {
                indicator.level = Levels.energy(sample.peak, sample.rms);
                indicator.phase = 0.32;
                wait(30);
                verify(indicator.counts[13] >= sample.minimum);
                compare(indicator.width, width);
                compare(indicator.height, height);
                compare(JSON.stringify(indicator.grid), grid);
                grabImage(indicator).save("build/listening-" + sample.name + ".png");
            }
            indicator.daemonState = "transcribing";
            compare(indicator.frameInterval, 80, "Processing cadence remains unchanged.");
            compare(indicator.counts, Matrix.capacities);
            compare(JSON.stringify(indicator.grid), grid);
            compare(indicator.cells.length, 162);
        }
    }
}
