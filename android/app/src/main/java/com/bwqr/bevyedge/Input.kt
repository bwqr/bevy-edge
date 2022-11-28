package com.bwqr.bevyedge

class Input {
    enum class KeyCode(val id: Int) {
        Up(0), Right(1), Down(2), Left(3),
    }

    companion object {
        fun init() {
            _init()
        }

        fun press(keyCode: KeyCode) {
            _press(keyCode.id)
        }

        fun release(keyCode: KeyCode) {
            _release(keyCode.id)
        }
    }
}

private external fun _init()
private external fun _press(id: Int)
private external fun _release(id: Int)