import QtQuick
import Quickshell
import Quickshell.Io
import qs.Common
import qs.Services
import qs.Widgets
import qs.Modules.Plugins
import "strings.js" as Strings

PluginComponent {
    id: root
    property bool autoRotateEnabled: true

    // User language via public API — same logic as DMS I18n's internal _rawLocale
    // (SessionData.locale, falling back to Qt.locale()). Avoids depending on the
    // private _resolvedLocale, whose name/semantics have already changed across DMS versions.
    readonly property string _lang: (SessionData.locale === "" ? Qt.locale().name : SessionData.locale).split(/[_-]/)[0]

    property string ccWidgetIcon: autoRotateEnabled ? "screen_rotation" : "screen_lock_rotation"
    property string ccWidgetPrimaryText: Strings.translate(root._lang, "ccWidgetPrimaryText")
    property string ccWidgetSecondaryText: Strings.translate(root._lang, autoRotateEnabled ? "secondaryAuto" : "secondaryLocked")
    property bool ccWidgetIsActive: autoRotateEnabled

    function updateState(content) {
        try {
            var s = JSON.parse(content);
            if (root.autoRotateEnabled !== s.auto_rotate) {
                ToastService.showInfo(Strings.translate(root._lang, s.auto_rotate ? "toastAutoRotateEnabled" : "toastLocked"));
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
