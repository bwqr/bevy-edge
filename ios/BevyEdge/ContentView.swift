import SwiftUI

struct ContentView: View {
    @GestureState private var upButton = false
    @GestureState private var downButton = false

    var body: some View {
        VStack {
            Text("Up")
            .gesture(
                DragGesture(minimumDistance: 0)
                    .updating($upButton, body: {(_, isTapped, _) in
                        if (!upButton) {
                            Input.press(Input.KeyCode.Up)
                        }

                        isTapped = true
                    })
                    .onEnded({ _ in
                        Input.release(Input.KeyCode.Up)
                    })
            )

            Text("Down")
            .gesture(
                DragGesture(minimumDistance: 0)
                    .updating($downButton, body: {(_, isTapped, _) in
                        if (!downButton) {
                            Input.press(Input.KeyCode.Down)
                        }

                        isTapped = true
                    })
                    .onEnded({ _ in
                        Input.release(Input.KeyCode.Down)
                    })
            )
        }
        .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}
