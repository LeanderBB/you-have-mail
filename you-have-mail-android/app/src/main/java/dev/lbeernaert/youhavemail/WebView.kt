package dev.lbeernaert.youhavemail

import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import android.webkit.ConsoleMessage
import android.webkit.JavascriptInterface
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material.Scaffold
import androidx.compose.material.Text
import androidx.compose.material.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.viewinterop.AndroidView

class WebView : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            // Calling the composable function
            // to display element and its contents
            MainContent()
        }
    }
}

// Creating a composable
// function to display Top Bar
@Composable
fun MainContent() {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("GFG | WebView", color = Color.White) },
                backgroundColor = Color(0xff0f9d58)
            )
        },
        content = { MyContent() }
    )
}

// Creating a composable
// function to create WebView
// Calling this function as
// content in the above function
@Composable
fun MyContent() {

    // Declare a string that contains a url

    // Adding a WebView inside AndroidView
    // with layout as full screen
    AndroidView(factory = {
        WebView(it).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
            webViewClient = object : WebViewClient() {
                override fun onReceivedError(
                    view: WebView?,
                    request: WebResourceRequest?,
                    error: WebResourceError?
                ) {
                    Log.e("XXX-WEBVIEW-XXX", "$error")
                    super.onReceivedError(view, request, error)
                }

                override fun onReceivedHttpError(
                    view: WebView?,
                    request: WebResourceRequest?,
                    errorResponse: WebResourceResponse?
                ) {
                    Log.e("XXX-WEBVIEW-XXX", "$errorResponse")
                    super.onReceivedHttpError(view, request, errorResponse)
                }
            }
            webChromeClient = object : WebChromeClient() {
                override fun onConsoleMessage(consoleMessage: ConsoleMessage?): Boolean {
                    Log.d("XXXX-CONSOLE-XXX", "${consoleMessage!!.message()}")
                    return super.onConsoleMessage(consoleMessage)
                }
            }
            addJavascriptInterface(object {
                @JavascriptInterface
                fun receiveMessage(data: String) {
                    this works!!
                    Log.d("XXXX-ANDROID-XXX", "data")
                }
            }, "AndroidInterface")
        }
    }, update = {
        it.settings.javaScriptEnabled = true;
        it.loadData(HTML, "text/html", null)
    })
}

val HTML = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>CAPTCHA</title>
</head>
<body>
<h1> HELLO </h1>
<script>
var postMessageToParent = function (message) {
    // Default on web clients
    window.parent.postMessage(message, '*');
}

var standalone = window.navigator.standalone;
var userAgent = window.navigator.userAgent.toLowerCase();
var safari = /safari/.test(userAgent);
var ios = /iphone|ipod|ipad/.test(userAgent);

    if (window.chrome && window.chrome.webview) {
        // This is an embedded chrome browser. It uses different message passing mechanism.
        client = 'webview';
        postMessageToParent = function (message) {
            chrome.webview.postMessage(message);
        }
    }

    if (window.webkit &&
        window.webkit.messageHandlers.linuxWebkitWebview) {
        // Webkit-GTK for Linux apps. NOTE: message handler can be named differently.
        client = 'webview';
        postMessageToParent = function (message) {
            window.webkit.messageHandlers.linuxWebkitWebview.postMessage(message);
        }
    }

    if (ios) {
        if (!standalone && safari) {
            //browser
        } else if (standalone && !safari) {
            //standalone
        } else if (!standalone && !safari) {
            //uiwebview
            client = 'ios';
        }
    }

    if (typeof AndroidInterface !== "undefined") {
        client = 'android';
    }

postMessageToParent("Hello world")
console.log("Hello")
console.log(AndroidInterface)
AndroidInterface.receiveMessage("Message")
</script>
</body>
</html>
"""