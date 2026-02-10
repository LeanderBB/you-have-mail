package dev.lbeernaert.youhavemail.screens

import android.net.http.SslError
import android.util.Log
import android.view.ViewGroup
import android.webkit.ConsoleMessage
import android.webkit.JavascriptInterface
import android.webkit.SslErrorHandler
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.components.AsyncScreen
import java.io.InputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale


const val PMCaptchaLogTag = "pm-captcha"

@Composable
fun ProtonCaptchaScreen(
    onBackClicked: () -> Unit,
    onCaptchaSuccess: suspend (String) -> Unit,
    onCaptchaFail: (String) -> Unit,
    html: String,
) {
    AsyncScreen(
        title = stringResource(id = R.string.captcha_request),
        onBackClicked = onBackClicked
    ) { padding, runTask ->
        Column(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize()
        ) {
            val taskLabel = stringResource(id = R.string.retry_login_with_captcha)
            AndroidView(factory = {
                WebView.setWebContentsDebuggingEnabled(true)
                WebView(it).apply {
                    layoutParams = ViewGroup.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT,
                        ViewGroup.LayoutParams.MATCH_PARENT
                    )
                    webViewClient = CaptchaWebView(
                        onCaptchaFail = onCaptchaFail,
                        html = html,
                    )
                    webChromeClient = object : WebChromeClient() {
                        override fun onConsoleMessage(consoleMessage: ConsoleMessage?): Boolean {
                            Log.d(PMCaptchaLogTag, "${consoleMessage!!.message()}")
                            return super.onConsoleMessage(consoleMessage)
                        }
                    }
                    addJavascriptInterface(object {
                        @JavascriptInterface
                        fun receiveResponse(data: String) {
                            Log.d(PMCaptchaLogTag, "Captcha solved")
                            // JSON expected by the backend.
                            val captchaResponse =
                                "{\"hv_type\":\"Captcha\", \"hv_token\":\"${data}\"}"
                            runTask(taskLabel) {
                                onCaptchaSuccess(captchaResponse)
                            }
                        }

                        @JavascriptInterface
                        fun receiveExpiredResponse(data: String) {
                            Log.d(PMCaptchaLogTag, "Received expired response:$data")
                            onCaptchaFail("Captcha expired")
                        }
                    }, "AndroidInterface")
                    settings.javaScriptEnabled = true
                    settings.allowContentAccess = false
                    settings.allowFileAccess = false
                    loadUrl("https://mail.proton.me/core/v4/captcha")
                }
            }, update = {
            })
        }
    }
}


class CaptchaWebView(var html: String, var onCaptchaFail: (String) -> Unit) :
    WebViewClient() {

    private val formatter: SimpleDateFormat = SimpleDateFormat("E, dd MMM yyyy kk:mm:ss", Locale.US)

    override fun onReceivedError(
        view: WebView?,
        request: WebResourceRequest?,
        error: WebResourceError?
    ) {
        super.onReceivedError(view, request, error)
        Log.e(PMCaptchaLogTag, "WebView Received Error: $error")
        onCaptchaFail("WebView Received Error: $error")
    }

    override fun onReceivedHttpError(
        view: WebView?,
        request: WebResourceRequest?,
        errorResponse: WebResourceResponse?
    ) {
        super.onReceivedHttpError(view, request, errorResponse)
        Log.e(PMCaptchaLogTag, "WebView Received HTTP Error: $errorResponse")
        onCaptchaFail("WebView Received HTTP Error: $errorResponse")
    }

    override fun onReceivedSslError(view: WebView?, handler: SslErrorHandler?, error: SslError?) {
        super.onReceivedSslError(view, handler, error)
        Log.e(PMCaptchaLogTag, "WebView Received SSL Error: $error")
        onCaptchaFail("WebView Received SSL Error: $error")
    }

    override fun shouldInterceptRequest(
        view: WebView?,
        request: WebResourceRequest
    ): WebResourceResponse? {
        if (request.url.schemeSpecificPart.startsWith("//mail.proton.me")) {
            Log.d(PMCaptchaLogTag, "Intercepting request")
            var headers = HashMap<String, String>()
            headers.put(
                "Access-Control-Allow-Origin", "*"
            )
            headers.put("Connection", "close")
            headers.put("Date", "${formatter.format(Date())} GMT")
            headers.put("Content-Type", "text/html")
            headers.put("Access-Control-Allow-Methods", "GET, POST, DELETE, PUT, OPTIONS")
            headers.put("Access-Control-Max-Age", "600")
            headers.put("Access-Control-Allow-Credentials", "true")
            headers.put("Access-Control-Allow-Headers", "accept, authorization, Content-Type")
            headers.put("Via", "you-have-mail")

            var reader: InputStream = html.byteInputStream()
            return WebResourceResponse(
                "text/html",
                "UTF-8",
                200,
                "OK",
                headers,
                reader,
            )
        }
        return super.shouldInterceptRequest(view, request)
    }
}