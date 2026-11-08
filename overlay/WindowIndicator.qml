import QtQuick
import "DotMatrix.js" as Matrix

Item {
    id: root

    property string daemonState: "recording"
    property real level: 0
    property bool simulate: false
    property bool animate: true
    property color accent: "#FFD60A"
    property int borderThickness: 2
    property real sizeMultiplier: 1.56
    property real devicePixelRatio: 1
    readonly property int bottomInset: Math.round(8 * sizeMultiplier)

    DotIndicator {
        objectName: "dictationIndicator"
        // This wrapper sits directly in the full-monitor scene. Snap the whole
        // scene-space origin, including the focused window's x/y offset.
        x: Matrix.snap(root.x + (root.width - width) / 2, devicePixelRatio) - root.x
        y: Matrix.snap(root.y + root.height - height - root.bottomInset, devicePixelRatio) - root.y
        // Do not shrink, distort, or spill out of tiny focused windows.
        visible: root.width >= width && root.height >= height + 2 * root.bottomInset
        accent: root.accent
        borderThickness: root.borderThickness
        sizeMultiplier: root.sizeMultiplier
        devicePixelRatio: root.devicePixelRatio
        daemonState: root.daemonState
        level: root.level
        simulate: root.simulate
        animate: root.animate
    }
}
