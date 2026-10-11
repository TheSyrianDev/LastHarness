import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine

ApplicationWindow {
    id: win
    visible: true
    width: 900
    height: 600
    title: "gt-qt"

    property string base: arg("--base", "http://127.0.0.1:7842")
    property string token: readToken(arg("--token-file", ""))
    property var ids: []          // [{display, id}]
    property var shown: []
    property string sessionId: ""
    property string sessionModel: ""
    property bool busy: false
    property var xhr: null
    property int selected: 0

    property bool rtl: arg("--rtl", "") === "1"
    LayoutMirroring.enabled: rtl
    LayoutMirroring.childrenInherit: true
    ListModel { id: lines; ListElement { t: "" } }
    // streamed text goes into the last paragraph; each newline starts a new one
    function addText(t) {
        var parts = t.split("\n")
        lines.setProperty(lines.count - 1, "t", lines.get(lines.count - 1).t + parts[0])
        for (var i = 1; i < parts.length; i++) lines.append({ t: parts[i] })
        view.positionViewAtEnd()
    }
    // a paragraph is right-to-left when its first letter is Arabic or Hebrew
    function isRtl(t) {
        var m = t.match(/[A-Za-z\u0590-\u08FF\uFB1D-\uFDFF\uFE70-\uFEFF]/)
        return m !== null && m[0].charCodeAt(0) >= 0x0590
    }
    function arg(name, def) {
        var a = Qt.application.arguments
        for (var i = 0; i < a.length - 1; i++) if (a[i] === name) return a[i + 1]
        return def
    }
    function readToken(p) {
        if (!p) return ""
        var x = new XMLHttpRequest()
        x.open("GET", "file://" + p, false)
        try { x.send() } catch (e) { return "" }
        return (x.responseText || "").trim()
    }
    function displayName(id) {
        var tiers = ["fast", "balanced", "max", "stock", "long", "low", "native", "high"]
        var s = id
        while (true) {
            var k = s.lastIndexOf("-")
            if (k < 0 || tiers.indexOf(s.substring(k + 1)) < 0) break
            s = s.substring(0, k)
        }
        return s
    }
    function idFor(display) {
        var all = ids.filter(function (m) { return m.display === display })
        var pick = all.filter(function (m) { return m.id.endsWith("-balanced") })
        return (pick[0] || all[0] || { id: display }).id
    }
    function call(method, path, body, done) {
        var x = new XMLHttpRequest()
        x.onreadystatechange = function () {
            if (x.readyState === XMLHttpRequest.DONE) done(x.status, x.responseText)
        }
        x.open(method, base + path)
        x.setRequestHeader("Authorization", "Bearer " + token)
        x.setRequestHeader("Content-Type", "application/json")
        x.send(body ? JSON.stringify(body) : undefined)
    }
    function loadModels() {
        call("GET", "/v1/models", null, function (st, txt) {
            try {
                var v = JSON.parse(txt)
                var arr = v.models || v.data || []
                var list = [], seen = [], out = []
                for (var i = 0; i < arr.length; i++) {
                    var id = typeof arr[i] === "string" ? arr[i] : arr[i].id
                    var d = displayName(id)
                    list.push({ display: d, id: id })
                    if (seen.indexOf(d) < 0) { seen.push(d); out.push(d) }
                }
                ids = list
                shown = out
            } catch (e) { addText("[error] " + e + "\n") }
        })
    }
    function send(text) {
        if (busy) { stop(); return }
        text = text.trim()
        if (!text) return
        input.text = ""
        addText("> " + text + "\n")
        busy = true
        var want = idFor(shown[selected] || "")
        function turn() {
            var x = new XMLHttpRequest()
            xhr = x
            var consumed = 0, event = ""
            function pump() {
                var t = x.responseText
                var end = t.lastIndexOf("\n")
                if (end < consumed) return
                var lines = t.substring(consumed, end + 1).split("\n")
                consumed = end + 1
                for (var i = 0; i < lines.length; i++) {
                    var l = lines[i]
                    if (l.indexOf("event: ") === 0) event = l.substring(7)
                    else if (l.indexOf("data: ") === 0 && event === "text") addText(JSON.parse(l.substring(6)).text)
                }
            }
            x.onreadystatechange = function () {
                if (x.readyState === 3) pump()
                if (x.readyState === 4) {
                    pump()
                    if (x.status >= 400) addText("\n[error] " + x.status + " " + x.responseText + "\n")
                    addText("\n")
                    busy = false
                    xhr = null
                }
            }
            x.open("POST", base + "/v1/sessions/" + sessionId + "/turns")
            x.setRequestHeader("Authorization", "Bearer " + token)
            x.setRequestHeader("Content-Type", "application/json")
            x.send(JSON.stringify({ text: text }))
        }
        if (sessionId === "") {
            call("POST", "/v1/sessions", { model: want, mode: "chat", max_tokens: 600 }, function (st, txt) {
                if (st >= 400) { addText("[error] " + st + " " + txt + "\n"); busy = false; return }
                sessionId = JSON.parse(txt).id
                sessionModel = want
                turn()
            })
        } else if (sessionModel !== want) {
            call("POST", "/v1/sessions/" + sessionId + "/model", { model: want }, function (st, txt) {
                if (st >= 400) { addText("[error] model switch " + st + "\n"); busy = false; return }
                sessionModel = want
                turn()
            })
        } else turn()
    }
    function stop() {
        if (xhr) xhr.abort()
        if (sessionId) call("POST", "/v1/sessions/" + sessionId + "/cancel", null, function () {})
        busy = false
    }
    Component.onCompleted: { loadModels(); input.forceActiveFocus() }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 4
        spacing: 8
        ColumnLayout {
            Layout.preferredWidth: 200
            Layout.maximumWidth: 200
            Layout.fillHeight: true
            Label { text: "Models"; font.bold: true }
            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: win.shown
                Accessible.role: Accessible.List
                Accessible.name: "Models"
                delegate: ItemDelegate {
                    width: ListView.view.width
                    text: modelData
                    highlighted: index === win.selected
                    onClicked: win.selected = index
                    Accessible.role: Accessible.ListItem
                    Accessible.name: modelData
                }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredWidth: 600
            Loader {
                id: three
                active: win.arg("--three", "") === "1"
                visible: active
                Layout.fillWidth: true
                Layout.preferredHeight: active ? 260 : 0
                sourceComponent: WebEngineView {
                    url: Qt.resolvedUrl("three/index.html")
                    onLoadingChanged: function (r) { console.log("web", r.status, r.errorString) }
                    onJavaScriptConsoleMessage: function (lv, msg, line, src) { console.log("js", msg, line) }
                }
            }
            ListView {
                id: view
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: lines
                Accessible.role: Accessible.StaticText
                Accessible.name: "Conversation"
                ScrollBar.vertical: ScrollBar {}
                delegate: Text {
                    width: ListView.view.width
                    text: t
                    wrapMode: Text.Wrap
                    color: palette.windowText
                    height: Math.max(implicitHeight, 8)
                    horizontalAlignment: win.isRtl(t) ? Text.AlignRight : Text.AlignLeft
                    LayoutMirroring.enabled: false
                }
            }
            RowLayout {
                TextField {
                    id: input
                    Layout.fillWidth: true
                    placeholderText: "Message"
                    Accessible.name: "Message"
                    onAccepted: win.send(text)
                }
                Button {
                    text: win.busy ? "Stop" : "Send"
                    onClicked: win.send(input.text)
                }
            }
        }
    }
}
