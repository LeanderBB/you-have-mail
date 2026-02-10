package dev.lbeernaert.youhavemail.components

import android.util.Log
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch

const val asyncScreenLogTag = "async"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AsyncScreen(
    title: String,
    onBackClicked: () -> Unit,
    content: @Composable (padding: PaddingValues, run: (String, suspend () -> Unit) -> Unit) -> Unit
) {
    val snackbarHostState = remember { SnackbarHostState() }
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
            snackbarHost = { SnackbarHost(snackbarHostState) },
            topBar = {
                TopAppBar(
                    title = {
                        Text(text = title)
                    },
                    navigationIcon = {
                        IconButton(onClick = onBackClicked) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Back"
                            )
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer,
                        titleContentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                        navigationIconContentColor = MaterialTheme.colorScheme.onPrimaryContainer
                    )
                )
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
                            snackbarHostState.showSnackbar(
                                message = e.message.orEmpty(),
                                duration = SnackbarDuration.Short,
                            )
                        }
                    } catch (err: Exception) {
                        openDialog.value = false
                        Log.e(asyncScreenLogTag, err.toString())
                        coroutineScope.launch {
                            snackbarHostState.showSnackbar(
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
