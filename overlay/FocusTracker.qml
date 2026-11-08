import QtQuick
import Quickshell.Hyprland
import Quickshell.Io
import "Geometry.js" as Geometry

QtObject {
    id: root
    property bool enabled: false
    property string focusedAddress: ""
    property int focusRevision: 0
    readonly property var client: {
        if (!enabled || !focusedAddress) return null;
        var windows = Hyprland.toplevels.values;
        for (var i = 0; i < windows.length; i++) {
            var ipc = windows[i].lastIpcObject;
            if (Geometry.normalizedAddress(ipc.address) === focusedAddress)
                return ipc;
        }
        return null;
    }

    function seedFocus() {
        if (!bootstrap || bootstrap.running) return;
        bootstrap.requestRevision = focusRevision;
        bootstrap.running = true;
    }

    onEnabledChanged: if (enabled) seedFocus()
    Component.onCompleted: if (enabled) seedFocus()

    // Quickshell 0.3.2 only fills activeToplevel after an activewindowv2 event,
    // and does not clear it for an empty event. Bootstrap once per activation,
    // then use the authoritative focus events, including the empty case.
    property Process bootstrap: Process {
        property int requestRevision: 0
        command: ["hyprctl", "-j", "activewindow"]
        stdout: StdioCollector {
            onStreamFinished: {
                // Never overwrite a newer focus event with an older IPC reply.
                if (!root.enabled || root.focusRevision !== root.bootstrap.requestRevision) return;
                try {
                    root.focusedAddress = Geometry.normalizedAddress(JSON.parse(text).address);
                } catch (error) {
                    root.focusedAddress = "";
                    console.warn("Voxtype UI: unable to read initial focus:", error);
                }
            }
        }
    }

    // Geometry changes are not all broadcast over the event socket. Refresh only
    // while the overlay is active; native IPC avoids spawning a process per tick.
    property Timer geometryTimer: Timer {
        interval: 80
        repeat: true
        running: root.enabled
        triggeredOnStart: true
        onTriggered: Hyprland.refreshToplevels()
    }

    property Connections events: Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name === "activewindowv2") {
                root.focusRevision++;
                root.focusedAddress = Geometry.normalizedAddress(event.data);
                if (root.enabled) Hyprland.refreshToplevels();
            } else if (event.name === "closewindow"
                       && Geometry.normalizedAddress(event.data) === root.focusedAddress) {
                root.focusRevision++;
                root.focusedAddress = "";
            }
            if (root.enabled && /^(monitor|configreloaded)/.test(event.name))
                Hyprland.refreshMonitors();
        }
    }
}
