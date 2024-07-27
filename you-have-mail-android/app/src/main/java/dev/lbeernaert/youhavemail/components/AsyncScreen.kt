package dev.lbeernaert.youhavemail.components

import android.util.Log
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.material.Icon
import androidx.compose.material.IconButton
import androidx.compose.material.Scaffold
import androidx.compose.material.SnackbarDuration
import androidx.compose.material.Text
import androidx.compose.material.TopAppBar
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.rememberScaffoldState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch

const val asyncScreenLogTag = "async"

@Composable
fun AsyncScreen(
    title: String,
    onBackClicked: () -> Unit,
    content: @Composable (padding: PaddingValues, run: (String, suspend () -> Unit) -> Unit) -> Unit
) {
    val scaffoldState = rememberScaffoldState()
    val openDialog = remember { mutableStateOf(false) }
    val coroutineScope = rememberCoroutineScope()
    val backgroundText = remember {
        mutableStateOf("")
    }

    val setAsyncTaskLabel: (String) -> Unit = {
        backgroundText.value = it
    }

    if (openDialog.value) {
        BackgroundTask(
            text = backgroundText.value
        )
    } else {
        Scaffold(
            scaffoldState = scaffoldState,
            topBar = {
                TopAppBar(title = {
                    Text(text = title)
                },
                    navigationIcon = {
                        IconButton(onClick = onBackClicked) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Back"
                            )
                        }
                    })
            }
        ) { padding ->
            content(padding) { label, run ->
                coroutineScope.launch {
                    setAsyncTaskLabel(label)
                    openDialog.value = true
                    try {
                        run()
                    } catch (e: Exception) {
                        openDialog.value = false
                        Log.e(asyncScreenLogTag, e.toString())
                        coroutineScope.launch {
                            scaffoldState.snackbarHostState.showSnackbar(
                                message = e.message.orEmpty(),
                                duration = SnackbarDuration.Short,
                            )
                        }
                    } catch (err: Exception) {
                        openDialog.value = false
                        Log.e(asyncScreenLogTag, err.toString())
                        coroutineScope.launch {
                            scaffoldState.snackbarHostState.showSnackbar(
                                message = "Unknown Error",
                                duration = SnackbarDuration.Short,
                            )
                        }
                    } finally {
                        openDialog.value = false
                    }
                }
            }
        }
    }
}