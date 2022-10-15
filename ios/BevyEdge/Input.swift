class Input {
    enum KeyCode: Int32 {
        case Up = 0, Down = 1
    }

    static func initialize() {
        input_init()
    }

    static func press(_ button: KeyCode) {
        input_press(button.rawValue)
    }

    static func release(_ button: KeyCode) {
        input_release(button.rawValue)
    }
}
