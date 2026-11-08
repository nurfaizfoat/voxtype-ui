import QtQuick
import "../overlay"

Rectangle {
    id: root
    property real renderScale: 1
    width: 240
    height: 120
    color: "#0D0D0D"

    WindowIndicator {
        objectName: "windowIndicator"
        x: 12; y: 34; width: 205; height: 60
        accent: "#6e9cf7"
        borderThickness: 2
        animate: false
        devicePixelRatio: root.renderScale
    }
}
