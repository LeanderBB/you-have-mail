package dev.lbeernaert.youhavemail.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.AutofillNode
import androidx.compose.ui.autofill.AutofillType
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalAutofill
import androidx.compose.ui.platform.LocalAutofillTree

// This file is adapted from https://medium.com/@bagadeshrp/compose-ui-textfield-autofill-6e2ac434e380

// The autofill modifier internally adds two modifiers:
// one to setup the layout, and one to listen for focus events.
@OptIn(ExperimentalComposeUiApi::class)
fun Modifier.autofill(handler: AutoFillHandler): Modifier {
    return this.then(
        onGloballyPositioned {
            handler.autoFillNode.boundingBox = it.boundsInWindow()
        }
    ).then(
        onFocusChanged {
            if (it.isFocused) {
                handler.request()
            } else {
                handler.cancel()
            }
        }
    )
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun AutoFillRequestHandler(
    autofillTypes: List<AutofillType> = listOf(),
    onFill: (String) -> Unit,
): AutoFillHandler {
    val autoFillNode = remember {
        AutofillNode(
            autofillTypes = autofillTypes,
            onFill = { onFill(it) }
        )
    }
    val autofill = LocalAutofill.current
    LocalAutofillTree.current += autoFillNode
    return remember {
        object : AutoFillHandler {
            override val autoFillNode: AutofillNode
                get() = autoFillNode

            override fun request() {
                autofill?.requestAutofillForNode(autofillNode = autoFillNode)
            }

            override fun cancel() {
                autofill?.cancelAutofillForNode(autofillNode = autoFillNode)
            }
        }
    }
}

@OptIn(ExperimentalComposeUiApi::class)
interface AutoFillHandler {
    val autoFillNode: AutofillNode
    fun request()
    fun cancel()
}
