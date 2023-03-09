package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import dev.lbeernaert.youhavemail.Log
import dev.lbeernaert.youhavemail.ServiceException
import dev.lbeernaert.youhavemail.components.BackgroundTask
import kotlinx.coroutines.launch


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
                        Log.e(err.toString())
                        coroutineScope.launch {
                            scaffoldState.snackbarHostState.showSnackbar(
                                message = err.message.toString(),
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