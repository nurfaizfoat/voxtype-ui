import QtQuick

Item {
    id: root
    readonly property string modulePath: "file:/usr/share/voxtype/quickshell/voxtype-shared/"
    readonly property string daemonState: stateLoader.item ? stateLoader.item.state : "idle"
    readonly property bool suppressed: stateLoader.item ? stateLoader.item.osdSuppressed : true
    property real peak: 0
    property real rms: 0
    property double lastFrameAt: 0

    // Load the packaged objects by URL instead of importing their directory.
    // This avoids Quickshell's config-relative scanner rewriting a file: import.
    Loader {
        id: stateLoader
        source: root.modulePath + "StateReader.qml"
        onStatusChanged: if (status === Loader.Error) console.error("Voxtype UI: unable to load Voxtype StateReader.");
    }
    Loader {
        id: audioLoader
        source: root.modulePath + "AudioBridge.qml"
        onStatusChanged: if (status === Loader.Error) console.error("Voxtype UI: unable to load Voxtype AudioBridge.");
    }
    Connections {
        target: audioLoader.item
        function onFrameReceived(peak, rms, vad, tsMs) {
            root.peak = peak;
            root.rms = rms;
            root.lastFrameAt = Date.now();
        }
        function onDisconnected() {
            root.peak = 0;
            root.rms = 0;
            root.lastFrameAt = 0;
        }
    }
}
