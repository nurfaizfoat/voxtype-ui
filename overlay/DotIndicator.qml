import QtQuick
import "DotMatrix.js" as Matrix

// Pure QtQuick visual: no compositor, audio capture, fonts, or theme dependencies.
Rectangle {
    id: root

    property bool animate: true
    property bool simulate: true
    property string daemonState: "recording"
    property real level: 0
    property real phase: 0
    property int pixelScale: 1
    property real sizeMultiplier: 1.56 // Previous 1.3× size, enlarged another 20%.
    // The embedding WINDOW supplies this. Screen.devicePixelRatio can be 2
    // while a fractional Wayland window actually renders at 1.25.
    property real devicePixelRatio: 1
    property color accent: "#FFD60A"
    property int borderThickness: 1
    readonly property int integerScale: Math.max(1, pixelScale)
    readonly property real unit: integerScale * Math.max(1, sizeMultiplier)
    readonly property real renderRatio: devicePixelRatio > 0 ? devicePixelRatio : 1
    readonly property int physicalWidth: Math.round(width * renderRatio)
    readonly property int physicalHeight: Math.round(height * renderRatio)
    readonly property int physicalBorder: Math.round(border.width * renderRatio)
    readonly property var grid: Matrix.gridLayout(unit * renderRatio, physicalBorder, Matrix.bufferQuantum(renderRatio))
    readonly property bool processing: daemonState === "transcribing"
    readonly property real amplitude: simulate ? Matrix.speechEnvelope(phase) : level
    // Disabling animation provides the exact reference silhouette for inspection.
    readonly property var counts: !animate || processing ? Matrix.capacities
        : Matrix.activeCounts(daemonState === "idle" ? 0 : amplitude, phase)
    readonly property var cells: Matrix.cells(grid, counts, processing, phase)

    implicitWidth: grid.width / renderRatio
    implicitHeight: grid.height / renderRatio
    width: implicitWidth
    height: implicitHeight
    color: "#0D0D0D"
    // Keep the compositor's logical border thickness, independent of UI scaling.
    border.width: Math.max(0, borderThickness) * integerScale
    border.color: accent
    radius: 0
    antialiasing: false
    clip: true

    Canvas {
        id: raster
        // Paint integer rectangles once in a native-pixel image. Individual
        // scene-graph rectangles can acquire an extra edge at fractional DPR.
        anchors.fill: parent
        renderTarget: Canvas.Image
        smooth: false
        antialiasing: false
        onPaint: {
            var ctx = getContext("2d");
            ctx.reset();
            // Canvas's backing image already has the window DPR. Cancel that
            // logical-coordinate transform so fillRect uses physical pixels.
            ctx.scale(1 / root.renderRatio, 1 / root.renderRatio);
            ctx.fillStyle = root.accent;
            ctx.fillRect(0, 0, root.physicalWidth, root.physicalHeight);
            var border = root.physicalBorder;
            ctx.fillStyle = root.color;
            ctx.fillRect(border, border, root.physicalWidth - 2 * border, root.physicalHeight - 2 * border);
            ctx.fillStyle = root.accent;
            for (var cell of root.cells) {
                ctx.globalAlpha = cell.opacity;
                ctx.fillRect(cell.x, cell.y, cell.width, cell.height);
            }
        }
    }

    onCellsChanged: raster.requestPaint()
    onAccentChanged: raster.requestPaint()
    onColorChanged: raster.requestPaint()

    Timer {
        interval: 80
        repeat: true
        running: root.animate && root.visible
        onTriggered: root.phase += interval / 1000
    }
}
