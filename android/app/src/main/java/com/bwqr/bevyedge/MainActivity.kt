package com.bwqr.bevyedge

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import com.bwqr.bevyedge.theme.BevyEdgeTheme

private var BevyedgeInitted = false

private fun init() {
    if (BevyedgeInitted) {
        return
    }

    BevyedgeInitted = true

    System.loadLibrary("bevyedge")
    Input.init()
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        init()

        setContent {
            BevyEdgeTheme {
                // A surface container using the 'background' color from the theme
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colors.background
                ) {
                    MainScreen()
                }
            }
        }
    }
}

@Composable
fun MainScreen() {
    Column {
        Text("Left", modifier = Modifier.pointerInput(Unit) {
            detectTapGestures(
                onPress = {
                    Input.press(Input.KeyCode.Left)
                    tryAwaitRelease()
                    Input.release(Input.KeyCode.Left)
                }
            )
        })

        Text("Right", modifier = Modifier.pointerInput(Unit) {
            detectTapGestures(
                onPress = {
                    Input.press(Input.KeyCode.Right)
                    tryAwaitRelease()
                    Input.release(Input.KeyCode.Right)
                }
            )
        })
    }
}