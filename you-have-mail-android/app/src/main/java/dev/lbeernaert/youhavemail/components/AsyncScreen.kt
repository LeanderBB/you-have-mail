package dev.lbeernaert.youhavemail.components

import android.util.Log
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import dev.lbeernaert.youhavemail.ServiceException
import dev.lbeernaert.youhavemail.serviceExceptionToErrorStr
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
                                imageVector = Icons.Filled.ArrowBack,
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
                    } catch (err: ServiceException) {
                        openDialog.value = false
                        Log.e(asyncScreenLogTag, err.toString())
                        coroutineScope.launch {
                            scaffoldState.snackbarHostState.showSnackbar(
                                message = serviceExceptionToErrorStr(err, null),
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