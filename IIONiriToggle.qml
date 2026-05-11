import QtQuick
import Quickshell
import Quickshell.Io
import qs.Common
import qs.Services
import qs.Widgets
import qs.Modules.Plugins

PluginComponent {
    id: root
    property bool autoRotateEnabled: true

    Process {
        id: stateReader
        command: ["tail", "-n", "1", "-f", "/var/lib/iio-niri-toggle/state.json"]
        running: true
        stdout: SplitParser {
            onRead: {
                try {
                    var s = JSON.parse(data);
                    root.autoRotateEnabled = s.auto_rotate;
                } catch (e) {}
            }
        }
    }

    function toggleRotation() {
        root.autoRotateEnabled = !root.autoRotateEnabled;
        if (root.autoRotateEnabled) {
            Quickshell.execDetached(["/usr/local/bin/iio-niri-toggle", "unlock"]);
            ToastService.showInfo("屏幕自动旋转已开启");
        } else {
            Quickshell.execDetached(["/usr/local/bin/iio-niri-toggle", "lock"]);
            ToastService.showInfo("屏幕方向已锁定");
        }
    }

    horizontalBarPill: Component {
        Row {
            spacing: Theme.spacingS
            StyledRect {
                width: 24; height: 24; radius: 4
                DankIcon {
                    name: root.autoRotateEnabled ? "screen_rotation" : "screen_lock_rotation"
                    size: Theme.iconSize
                }
                MouseArea {
                    anchors.fill: parent; hoverEnabled: true
                    onClicked: root.toggleRotation()
                }
            }
        }
    }

    verticalBarPill: Component {
        Column {
            spacing: Theme.spacingXS
            StyledRect {
                width: 24; height: 24; radius: 4
                DankIcon {
                    name: root.autoRotateEnabled ? "screen_rotation" : "screen_lock_rotation"
                    size: Theme.iconSize
                }
                MouseArea {
                    anchors.fill: parent; hoverEnabled: true
                    onClicked: root.toggleRotation()
                }
            }
        }
    }
}
