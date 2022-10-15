import SwiftUI

@main
struct BevyEdgeApp: App {

    init() {
        Input.initialize()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
