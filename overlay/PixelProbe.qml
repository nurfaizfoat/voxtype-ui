import QtQuick
import QtQuick.Window
import Quickshell
import Quickshell.Wayland

ShellRoot {
    readonly property bool capture: Quickshell.env("VOXTYPE_UI_CAPTURE_PROBE") === "1"
    PanelWindow {
        id: surface
        color: "transparent"
        anchors { top: true; bottom: true; left: true; right: true }
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.namespace: "voxtype-ui-pixel-probe"
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        mask: Region { intersection: Intersection.Subtract; width: surface.width; height: surface.height }

        Item {
            id: content
            DotIndicator {
                id: indicator
                x: 100; y: 100
                visible: capture
                animate: false
                accent: "#6e9cf7"
                borderThickness: 2
                devicePixelRatio: surface.devicePixelRatio
            }
            Timer {
                interval: 700
                running: true
                onTriggered: {
                    console.log("VOXTYPE_PIXEL_PROBE", JSON.stringify({
                        screenRatio: content.Screen.devicePixelRatio, windowRatio: surface.devicePixelRatio,
                        screenName: surface.screen.name,
                        x: indicator.x, y: indicator.y, width: indicator.width, height: indicator.height,
                        border: indicator.physicalBorder, accent: indicator.accent.toString(), grid: indicator.grid
                    }));
                    if (!capture) Qt.quit();
                }
            }
        }
    }
}
