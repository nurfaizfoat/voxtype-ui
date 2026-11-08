import QtQuick
import Quickshell
import Quickshell.Io
import "Geometry.js" as Geometry

QtObject {
    id: root
    property bool enabled: false
    property color accent: "#FFD60A"
    property int cornerRadius: 0
    property int borderThickness: 2

    property FileView colors: FileView {
        path: Quickshell.env("HOME") + "/.local/state/omarchy/current/theme/colors.toml"
        watchChanges: true
        printErrors: false
        onLoaded: root.accent = Geometry.accentFromToml(text(), "#FFD60A")
        onLoadFailed: root.accent = "#FFD60A"
        onFileChanged: reload()
    }

    property Process rounding: Process {
        command: ["hyprctl", "-j", "getoption", "decoration:rounding"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var value = JSON.parse(text).int;
                    if (typeof value === "number" && isFinite(value))
                        root.cornerRadius = Math.max(0, value);
                } catch (error) {
                    console.warn("Voxtype UI: unable to read window rounding:", error);
                }
            }
        }
    }

    property Process borderSize: Process {
        command: ["hyprctl", "-j", "getoption", "general:border_size"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var value = JSON.parse(text).int;
                    if (typeof value === "number" && isFinite(value))
                        root.borderThickness = Math.max(0, Math.round(value));
                } catch (error) {
                    console.warn("Voxtype UI: unable to read window border size:", error);
                }
            }
        }
    }

    // Theme changes replace symlinks as well as files. Periodic reload re-arms
    // the FileView watch even if its previous inode disappeared during a switch.
    property Timer refresh: Timer {
        interval: 2000
        repeat: true
        triggeredOnStart: true
        running: root.enabled
        onTriggered: {
            root.colors.reload();
            if (!root.rounding.running) root.rounding.running = true;
            if (!root.borderSize.running) root.borderSize.running = true;
        }
    }
}
