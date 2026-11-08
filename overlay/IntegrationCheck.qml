import QtQuick
import Quickshell
import Quickshell.Hyprland

ShellRoot {
    FocusTracker { id: focus; enabled: true }
    Appearance { id: appearance; enabled: true }
    VoxtypeSource { id: source }

    Timer {
        interval: 1000
        running: true
        onTriggered: {
            console.log("Voxtype UI integration loaded; state:", source.daemonState,
                        "has focused client:", !!focus.client,
                        "tracked windows:", Hyprland.toplevels.values.length,
                        "tracked monitors:", Hyprland.monitors.values.length,
                        "accent:", appearance.accent,
                        "border thickness:", appearance.borderThickness,
                        "rounding:", appearance.cornerRadius);
            Qt.quit();
        }
    }
}
