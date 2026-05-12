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

    property string ccWidgetIcon: autoRotateEnabled ? "screen_rotation" : "screen_lock_rotation"
    property string ccWidgetPrimaryText: "屏幕旋转"
    property string ccWidgetSecondaryText: autoRotateEnabled ? "自动旋转" : "已锁定"
    property bool ccWidgetIsActive: autoRotateEnabled

    function updateState(content) {
        try {
            var s = JSON.parse(content);
            if (root.autoRotateEnabled !== s.auto_rotate) {
                if (s.auto_rotate) {
                    ToastService.showInfo("屏幕自动旋转已开启");
                } else {
                    ToastService.showInfo("屏幕方向已锁定");
                }
            }
            root.autoRotateEnabled = s.auto_rotate;
        } catch (e) {}
    }

    FileView {
        id: stateFile
        path: "/var/lib/iio-niri-toggle/state.json"
        preload: true
        blockLoading: true
        watchChanges: true
        onLoaded: root.updateState(stateFile.text())
        onFileChanged: {
            stateFile.reload()
            root.updateState(stateFile.text())
        }
    }

    function toggleRotation() {
        if (root.autoRotateEnabled) {
            Quickshell.execDetached(["/usr/local/bin/iio-niri-toggle", "lock"]);
        } else {
            Quickshell.execDetached(["/usr/local/bin/iio-niri-toggle", "unlock"]);
        }
    }

    onCcWidgetToggled: toggleRotation()

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
