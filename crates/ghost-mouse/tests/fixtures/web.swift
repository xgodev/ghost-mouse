// Test window: a web page with one button, behind everything, records what the page receives, quits by itself.
import AppKit
import WebKit

let logPath = CommandLine.arguments[1]
func log(_ s: String) {
    let line = s + "\n"
    if let h = FileHandle(forWritingAtPath: logPath) {
        h.seekToEndOfFile(); h.write(line.data(using: .utf8)!); try? h.close()
    } else {
        FileManager.default.createFile(atPath: logPath, contents: line.data(using: .utf8))
    }
}

final class Page: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
    func userContentController(_ c: WKUserContentController, didReceive m: WKScriptMessage) { log("\(m.body)") }
    func webView(_ v: WKWebView, didFinish n: WKNavigation!) { log("ready") }
}

let html = """
<body style="margin:0"><button id="b" style="position:absolute;left:100px;top:80px;width:160px;height:60px">Start</button>
<script>
const say = t => window.webkit.messageHandlers.log.postMessage(t);
for (const k of ['mousedown', 'mouseup', 'click', 'mouseover'])
  document.getElementById('b').addEventListener(k, () => say('page ' + k));
</script></body>
"""

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let w = NSWindow(contentRect: NSRect(x: 40, y: 40, width: 360, height: 240), styleMask: [.titled], backing: .buffered, defer: false)
w.title = "ghost-mouse-web"
w.acceptsMouseMovedEvents = true
let page = Page()
let config = WKWebViewConfiguration()
config.userContentController.add(page, name: "log")
let v = WKWebView(frame: w.contentView!.bounds, configuration: config)
v.navigationDelegate = page
w.contentView = v
v.loadHTMLString(html, baseURL: nil)
w.orderBack(nil)
NotificationCenter.default.addObserver(forName: NSApplication.didBecomeActiveNotification, object: nil, queue: nil) { _ in log("became-active") }
Timer.scheduledTimer(withTimeInterval: 60, repeats: false) { _ in exit(0) }
app.run()
