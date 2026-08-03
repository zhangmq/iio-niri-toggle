.pragma library

var strings = {
    en: {
        ccWidgetPrimaryText: "Screen Rotation",
        secondaryAuto: "Auto-rotate",
        secondaryLocked: "Locked",
        toastAutoRotateEnabled: "Screen auto-rotate enabled",
        toastLocked: "Screen orientation locked"
    },
    zh: {
        ccWidgetPrimaryText: "屏幕旋转",
        secondaryAuto: "自动旋转",
        secondaryLocked: "已锁定",
        toastAutoRotateEnabled: "屏幕自动旋转已开启",
        toastLocked: "屏幕方向已锁定"
    }
};

function translate(lang, key) {
    var table = strings[lang] || strings.en;
    if (table[key] !== undefined)
        return table[key];
    return strings.en[key] !== undefined ? strings.en[key] : key;
}
