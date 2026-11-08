import QtQuick
import QtTest
import "../overlay/Geometry.js" as Geometry

TestCase {
    name: "OverlayGeometry"

    function client(x, y, width, height) {
        return { address: "0x123", mapped: true, hidden: false,
                 at: [x, y], size: [width, height] };
    }

    function test_fractionalScaleUsesLogicalCoordinates() {
        var rect = Geometry.windowRect(client(12, 34, 1512, 818), 0, 0, 1536, 864);
        compare(rect.x, 12);
        compare(rect.y, 34);
        compare(rect.width, 1512);
        compare(rect.height, 818);
    }

    function test_negativeMonitorOrigin() {
        var rect = Geometry.windowRect(client(-1800, 100, 900, 600), -1920, 0, 1920, 1080);
        compare(rect.x, 120);
        compare(rect.y, 100);
    }

    function test_spanningWindowPreservesRealEdges() {
        var window = client(1400, 100, 500, 600);
        var left = Geometry.windowRect(window, 0, 0, 1536, 864);
        var right = Geometry.windowRect(window, 1536, 0, 1920, 1080);
        compare(left.width, 500);
        compare(right.x, -136);
        compare(right.width, 500);
    }

    function test_fullscreenAndPortrait() {
        var rect = Geometry.windowRect(client(1536, -200, 1080, 1920), 1536, -200, 1080, 1920);
        compare(rect.x, 0);
        compare(rect.y, 0);
        compare(rect.width, 1080);
        compare(rect.height, 1920);
    }

    function test_invalidOrHiddenWindow() {
        compare(Geometry.windowRect(null, 0, 0, 100, 100), null);
        compare(Geometry.windowRect({}, 0, 0, 100, 100), null);
        var window = client(0, 0, 100, 100);
        window.hidden = true;
        compare(Geometry.windowRect(window, 0, 0, 100, 100), null);
        window.hidden = false;
        window.mapped = false;
        compare(Geometry.windowRect(window, 0, 0, 100, 100), null);
        window.mapped = true;
        window.visible = false;
        compare(Geometry.windowRect(window, 0, 0, 100, 100), null);
        compare(Geometry.windowRect(client(0, 0, 0, 100), 0, 0, 100, 100), null);
        compare(Geometry.windowRect(client(NaN, 0, 10, 10), 0, 0, 100, 100), null);
        compare(Geometry.windowRect(client(0, 0, Infinity, 10), 0, 0, 100, 100), null);
    }

    function test_otherMonitorAndTouchingEdge() {
        compare(Geometry.windowRect(client(100, 0, 100, 100), 0, 0, 100, 100), null);
        compare(Geometry.windowRect(client(-100, 0, 100, 100), 0, 0, 100, 100), null);
        compare(Geometry.windowRect(client(0, 100, 100, 100), 0, 0, 100, 100), null);
    }

    function test_audioClamping() {
        compare(Geometry.energy(0, 0), 0);
        compare(Geometry.energy(-1, -1), 0);
        compare(Geometry.energy(1, 1), 1);
        compare(Geometry.energy(NaN, 0), 0);
        compare(Geometry.energy(0, Infinity), 0);
        verify(Geometry.energy(0.1, 0.05) > 0);
    }

    function test_focusAddressesAndEmptyFocus() {
        compare(Geometry.normalizedAddress("0xAb123"), "ab123");
        compare(Geometry.normalizedAddress("ab123"), "ab123");
        compare(Geometry.normalizedAddress(""), "");
        compare(Geometry.normalizedAddress("0x0"), "");
        compare(Geometry.normalizedAddress(undefined), "");
        compare(Geometry.normalizedAddress("not a window"), "");
    }

    function test_screensaverIsNeverHighlighted() {
        var window = client(0, 0, 100, 100);
        window.class = "org.omarchy.screensaver";
        compare(Geometry.windowRect(window, 0, 0, 100, 100), null);
    }

    function test_historyIsBoundedAndImmutable() {
        var samples = [0.1, 0.2, 0.3];
        var next = Geometry.appendSample(samples, 2, 3);
        compare(next.length, 3);
        compare(next[0], 0.2);
        compare(next[2], 1);
        compare(samples[0], 0.1);
        compare(Geometry.appendSample(samples, 0.5, 1).length, 1);
    }

    function test_accentParsing() {
        compare(Geometry.accentFromToml('accent = "#6e9cf7" # Theme\n', "#FFD60A"), "#6e9cf7");
        compare(Geometry.accentFromToml("accent = '#aabbcc'", "#FFD60A"), "#aabbcc");
        compare(Geometry.accentFromToml('accent = "oops"', "#FFD60A"), "#FFD60A");
        compare(Geometry.accentFromToml('# accent = "#123456"', "#FFD60A"), "#FFD60A");
    }
}
