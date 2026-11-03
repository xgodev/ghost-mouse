// iPad test window run on the Mac (Mac Catalyst): a text field, a long list and a button. Records what
// UIKit hands them, stays behind everything, quits by itself.
import UIKit

let logPath = ProcessInfo.processInfo.environment["GHOST_TARGET_LOG"]!
func log(_ s: String) {
    let line = s + "\n"
    if let h = FileHandle(forWritingAtPath: logPath) {
        h.seekToEndOfFile(); h.write(line.data(using: .utf8)!); try? h.close()
    } else {
        FileManager.default.createFile(atPath: logPath, contents: line.data(using: .utf8))
    }
}

final class Field: UITextField {
    override func becomeFirstResponder() -> Bool {
        let took = super.becomeFirstResponder()
        log("field focused=\(took)")
        return took
    }
}

final class Screen: UIViewController, UITableViewDelegate, UITableViewDataSource {
    let field = Field(frame: CGRect(x: 20, y: 20, width: 300, height: 34))
    let list = UITableView(frame: CGRect(x: 20, y: 80, width: 300, height: 200))

    override func viewDidLoad() {
        view.backgroundColor = .systemGreen
        field.borderStyle = .roundedRect
        field.accessibilityLabel = "Search"
        field.addTarget(self, action: #selector(edited), for: .editingChanged)
        view.addSubview(field)
        list.dataSource = self
        list.delegate = self
        list.register(UITableViewCell.self, forCellReuseIdentifier: "row")
        view.addSubview(list)
        let button = UIButton(type: .system)
        button.setTitle("Go", for: .normal)
        button.frame = CGRect(x: 20, y: 290, width: 80, height: 24)
        button.addAction(UIAction { _ in log("pressed Go") }, for: .primaryActionTriggered)
        view.addSubview(button)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) { log("ready") }
        DispatchQueue.main.asyncAfter(deadline: .now() + 60) { exit(0) }
    }

    @objc func edited() { log("text=\(field.text ?? "")") }
    func tableView(_ t: UITableView, numberOfRowsInSection s: Int) -> Int { 200 }
    func tableView(_ t: UITableView, cellForRowAt i: IndexPath) -> UITableViewCell {
        let cell = t.dequeueReusableCell(withIdentifier: "row", for: i)
        cell.textLabel?.text = "Row \(i.row)"
        return cell
    }
    func tableView(_ t: UITableView, didSelectRowAt i: IndexPath) { log("selected Row \(i.row)") }
    func scrollViewDidScroll(_ s: UIScrollView) { log("scrolled y=\(Int(s.contentOffset.y))") }
}

final class Scene: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    func scene(_ scene: UIScene, willConnectTo s: UISceneSession, options: UIScene.ConnectionOptions) {
        guard let windowScene = scene as? UIWindowScene else { return }
        windowScene.title = "ghost-mouse-catalyst"
        windowScene.sizeRestrictions?.minimumSize = CGSize(width: 360, height: 340)
        windowScene.sizeRestrictions?.maximumSize = CGSize(width: 360, height: 340)
        let window = UIWindow(windowScene: windowScene)
        window.rootViewController = Screen()
        window.makeKeyAndVisible()
        self.window = window
    }
}

final class Delegate: UIResponder, UIApplicationDelegate {
    func application(_ a: UIApplication, configurationForConnecting s: UISceneSession, options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(name: nil, sessionRole: s.role)
        configuration.delegateClass = Scene.self
        return configuration
    }
}

UIApplicationMain(CommandLine.argc, CommandLine.unsafeArgv, nil, NSStringFromClass(Delegate.self))
