import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland
import "Geometry.js" as Geometry

ShellRoot {
    id: root

    readonly property bool demo: Quickshell.env("VOXTYPE_UI_DEMO") === "1"
    property real demoTime: 0
    readonly property string daemonState: demo
        ? (demoTime % 10 < 7 ? "recording" : "transcribing")
        : (source.item ? source.item.daemonState : "idle")
    readonly property bool active: ["recording", "streaming", "transcribing"].indexOf(daemonState) >= 0
                                  && (demo || (source.item && !source.item.suppressed))
    property real level: 0

    Loader {
        id: source
        active: !root.demo
        source: "VoxtypeSource.qml"
        onStatusChanged: if (status === Loader.Error) console.error("Voxtype UI: failed to load the packaged Voxtype integration.");
    }

    FocusTracker { id: focus; enabled: root.active }
    Appearance { id: appearance; enabled: root.active }

    onActiveChanged: {
        root.level = 0;
        if (active) Hyprland.refreshMonitors();
    }

    Timer {
        interval: 80
        running: root.active
        repeat: true
        onTriggered: {
            if (root.demo) root.demoTime += interval / 1000;
            var value = 0;
            if (root.daemonState !== "transcribing") {
                if (!root.demo && source.item && Date.now() - source.item.lastFrameAt < 250) {
                    value = Geometry.energy(source.item.peak, source.item.rms);
                }
            }
            // Fast attack and slower decay keep quiet speech legible without
            // animating imaginary audio when the bridge stops sending frames.
            root.level += (value - root.level) * (value > root.level ? 0.7 : 0.25);
        }
    }

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: surface
            required property var modelData
            screen: modelData
            readonly property var monitor: Hyprland.monitorFor(modelData)
            readonly property var rect: monitor ? Geometry.windowRect(
                focus.client, monitor.x, monitor.y, modelData.width, modelData.height) : null

            visible: root.active && rect !== null
            color: "transparent"
            anchors { top: true; bottom: true; left: true; right: true }
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.namespace: root.demo ? "voxtype-ui-preview" : "voxtype-ui"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

            // No pixel in this full-screen surface may receive pointer input.
            mask: Region {
                intersection: Intersection.Subtract
                x: 0; y: 0
                width: surface.width
                height: surface.height
            }

            WindowIndicator {
                x: surface.rect ? surface.rect.x : 0
                y: surface.rect ? surface.rect.y : 0
                width: surface.rect ? surface.rect.width : 0
                height: surface.rect ? surface.rect.height : 0
                daemonState: root.daemonState
                accent: appearance.accent
                borderThickness: appearance.borderThickness
                devicePixelRatio: surface.devicePixelRatio
                level: root.level
                simulate: root.demo
                animate: surface.visible
            }
        }
    }
}
