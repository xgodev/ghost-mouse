// Test window: named controls, two with the same name and one scrolled out of view, behind everything; records
// what is pressed, quits by itself.
import AppKit

let logPath = CommandLine.arguments[1]
func log(_ s: String) {
    let line = s + "\n"
    if let h = FileHandle(forWritingAtPath: logPath) {
        h.seekToEndOfFile(); h.write(line.data(using: .utf8)!); try? h.close()
    } else {
        FileManager.default.createFile(atPath: logPath, contents: line.data(using: .utf8))
    }
}

final class Actions: NSObject {
    @objc func pressed(_ b: NSButton) {
        log(b.tag < 0 ? "checked \(b.title) \(b.state == .on)" : "pressed \(b.title) \(b.tag)")
    }
}
let actions = Actions()

func button(_ title: String, tag: Int = 0, x: CGFloat, top: CGFloat, in view: NSView, height: CGFloat) -> NSButton {
    let b = NSButton(title: title, target: actions, action: #selector(Actions.pressed(_:)))
    b.tag = tag
    b.frame = NSRect(x: x, y: height - top - 30, width: 120, height: 30)
    view.addSubview(b)
    return b
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let w = NSWindow(contentRect: NSRect(x: 40, y: 40, width: 360, height: 240), styleMask: [.titled], backing: .buffered, defer: false)
w.title = "ghost-mouse-controls"
let content = w.contentView!
let height = content.bounds.height
_ = button("Press me", x: 20, top: 20, in: content, height: height)
let remember = button("Remember", x: 160, top: 20, in: content, height: height)
remember.setButtonType(.switch)
remember.tag = -1
_ = button("Same", tag: 1, x: 20, top: 70, in: content, height: height)
_ = button("Same", tag: 2, x: 160, top: 70, in: content, height: height)
let scroll = NSScrollView(frame: NSRect(x: 20, y: 10, width: 320, height: 80))
let page = NSView(frame: NSRect(x: 0, y: 0, width: 300, height: 600))
_ = button("Far away", x: 10, top: 560, in: page, height: 600)
scroll.documentView = page
scroll.hasVerticalScroller = true
content.addSubview(scroll)
page.scroll(NSPoint(x: 0, y: 600 - 80))
w.orderBack(nil)
NotificationCenter.default.addObserver(forName: NSApplication.didBecomeActiveNotification, object: nil, queue: nil) { _ in log("became-active") }
log("ready")
Timer.scheduledTimer(withTimeInterval: 60, repeats: false) { _ in exit(0) }
app.run()
