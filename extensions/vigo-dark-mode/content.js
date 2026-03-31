// Vigo Dark Mode — Content Script
// Inverts colors on the page for a quick dark mode effect.
(function () {
    "use strict";

    var STYLE_ID = "vigo-dark-mode-style";

    function enable() {
        if (document.getElementById(STYLE_ID)) return;
        var style = document.createElement("style");
        style.id = STYLE_ID;
        style.textContent =
            "html { filter: invert(1) hue-rotate(180deg); }" +
            "img, video, canvas, svg { filter: invert(1) hue-rotate(180deg); }";
        document.documentElement.appendChild(style);
    }

    function disable() {
        var el = document.getElementById(STYLE_ID);
        if (el) el.remove();
    }

    function toggle() {
        if (document.getElementById(STYLE_ID)) {
            disable();
        } else {
            enable();
        }
    }

    // Apply on load.
    enable();

    // Listen for toggle messages from the browser action.
    if (typeof vigo !== "undefined" && vigo.runtime && vigo.runtime.onMessage) {
        vigo.runtime.onMessage.addListener(function (msg) {
            if (msg.action === "toggle") {
                toggle();
            }
        });
    }
})();
