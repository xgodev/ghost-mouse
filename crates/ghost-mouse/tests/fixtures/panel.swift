// Test program: stays behind everything, opens the native file panel and records what came back, quits by itself.
import AppKit
import UniformTypeIdentifiers

let logPath = CommandLine.arguments[1]
let folder = CommandLine.arguments[2]
let env = ProcessInfo.processInfo.environment
func log(_ s: String) {
    let line = s + "\n"
    if let h = FileHandle(forWritingAtPath: logPath) {
        h.seekToEndOfFile(); h.write(line.data(using: .utf8)!); try? h.close()
    } else {
        FileManager.default.createFile(atPath: logPath, contents: line.data(using: .utf8))
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let w = NSWindow(contentRect: NSRect(x: 40, y: 40, width: 720, height: 520), styleMask: [.titled], backing: .buffered, defer: false)
w.title = "ghost-mouse-panel"
w.orderBack(nil)
NotificationCenter.default.addObserver(forName: NSApplication.didBecomeActiveNotification, object: nil, queue: nil) { _ in log("became-active") }
DispatchQueue.global().asyncAfter(deadline: .now() + 60) { exit(0) }
DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
    let panel: NSSavePanel = env["GHOST_PANEL_SAVE"] != nil ? NSSavePanel() : NSOpenPanel()
    panel.directoryURL = URL(fileURLWithPath: folder)
    if env["GHOST_PANEL_ONLY_PNG"] != nil {
        panel.allowedContentTypes = [.png]
    }
    func answer(_ response: NSApplication.ModalResponse) {
        // The panel may decompose accents; the test compares against the composed form.
        log(response == .OK ? "chose \((panel.url?.path ?? "").precomposedStringWithCanonicalMapping)" : "cancelled")
    }
    log("panel-opening")
    if env["GHOST_PANEL_SHEET"] != nil {
        panel.beginSheetModal(for: w, completionHandler: answer)
    } else {
        answer(panel.runModal())
    }
}
app.run()
