// Test window: stays behind everything unless told to grab the front, records what its view receives, quits by itself.
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

func spot(_ e: NSEvent, _ w: NSWindow?) -> String {
    let height = w?.frame.height ?? 0
    return "x=\(Int(e.locationInWindow.x.rounded())) y=\(Int((height - e.locationInWindow.y).rounded()))"
}

final class V: NSView {
    override var acceptsFirstResponder: Bool { true }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    override func draw(_ r: NSRect) { NSColor.systemGreen.setFill(); bounds.fill() }
    override func mouseDown(with e: NSEvent) { log("down left \(spot(e, window)) clicks=\(e.clickCount)") }
    override func mouseUp(with e: NSEvent) { log("up left \(spot(e, window))") }
    override func rightMouseDown(with e: NSEvent) { log("down right \(spot(e, window)) clicks=\(e.clickCount)") }
    override func otherMouseDown(with e: NSEvent) { log("down middle \(spot(e, window)) button=\(e.buttonNumber)") }
    override func mouseDragged(with e: NSEvent) { log("drag \(spot(e, window))") }
    override func mouseMoved(with e: NSEvent) { log("moved \(spot(e, window))") }
    override func scrollWheel(with e: NSEvent) { log("scroll dx=\(Int(e.deltaX)) dy=\(Int(e.deltaY))") }
    override func keyDown(with e: NSEvent) { log("key chars=\(e.characters ?? "") code=\(e.keyCode) cmd=\(e.modifierFlags.contains(.command)) shift=\(e.modifierFlags.contains(.shift))") }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let w = NSWindow(contentRect: NSRect(x: 40, y: 40, width: 360, height: 240), styleMask: [.titled], backing: .buffered, defer: false)
w.title = "ghost-mouse-target"
w.acceptsMouseMovedEvents = true
let v = V(frame: w.contentView!.bounds)
w.contentView = v
w.makeFirstResponder(v)
w.orderBack(nil)
NotificationCenter.default.addObserver(forName: NSApplication.didBecomeActiveNotification, object: nil, queue: nil) { _ in log("became-active") }
log("ready")
log("library-path=\(ProcessInfo.processInfo.environment["DYLD_FALLBACK_LIBRARY_PATH"] ?? "")")
if ProcessInfo.processInfo.environment["GHOST_TARGET_GRAB"] != nil {
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
        app.setActivationPolicy(.regular)
        app.activate(ignoringOtherApps: true)
        w.makeKeyAndOrderFront(nil)
    }
}
Timer.scheduledTimer(withTimeInterval: 60, repeats: false) { _ in exit(0) }
app.run()
