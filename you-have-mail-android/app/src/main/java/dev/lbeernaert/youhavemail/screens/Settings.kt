package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.Divider
import androidx.compose.material.DropdownMenu
import androidx.compose.material.DropdownMenuItem
import androidx.compose.material.ExperimentalMaterialApi
import androidx.compose.material.ExposedDropdownMenuBox
import androidx.compose.material.ExposedDropdownMenuDefaults
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.material.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.toSize
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.app.State
import dev.lbeernaert.youhavemail.components.ActionButton
import dev.lbeernaert.youhavemail.components.AsyncScreen


@OptIn(ExperimentalMaterialApi::class)
@Composable
fun Settings(
    state: State,
    onBackClicked: () -> Unit,
    onPollIntervalUpdate: (ULong) -> Unit,
    onExportLogsClicked: () -> Unit,
) {
    AsyncScreen(
        title = stringResource(id = R.string.settings),
        onBackClicked = onBackClicked
    ) { _, runTask ->

        val pollIntervalValue by state.getPollInterval().collectAsState()
        val updatingPollIntervalLabel = stringResource(id = R.string.update_poll_interval)

        Column(
            modifier = Modifier
                .padding(20.dp)
                .fillMaxSize()
        ) {
            val timeIntervals =
                listOf(30UL, 60UL, 120UL, 180UL, 300UL, 600UL, 900UL, 1200UL, 1800UL, 3600UL)
            var expanded by remember { mutableStateOf(false) }
            var selectedIndex by remember { mutableIntStateOf(0) }
            // textFieldWidth is used to assign to DropDownMenu the same width as TextField
            var textFieldWidth by remember { mutableStateOf(Size.Zero)}

            val onPollIntervalModified: () -> Unit = {
                runTask(updatingPollIntervalLabel) {
                    onPollIntervalUpdate(timeIntervals[selectedIndex])
                }
            }

            Text(
                text = stringResource(id = R.string.poll_interval),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.h6,
                modifier = Modifier.fillMaxWidth()
            )
            Spacer(modifier = Modifier.padding(5.dp))
            Text(
                text = stringResource(id = R.string.poll_interval_desc),
                style = MaterialTheme.typography.subtitle1
            )

            Spacer(modifier = Modifier.padding(5.dp))

            ExposedDropdownMenuBox(
                modifier = Modifier
                    .fillMaxWidth(),
                expanded = expanded,
                onExpandedChange = {
                    expanded = !expanded
                }
            ) {
                TextField(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { expanded = !expanded }
                        .onGloballyPositioned { coordinates ->
                            textFieldWidth = coordinates.size.toSize()
                        },
                    readOnly = true,
                    enabled = true,
                    value = secondsToText(pollIntervalValue),
                    onValueChange = { },
                    trailingIcon = {
                        ExposedDropdownMenuDefaults.TrailingIcon(
                            expanded = expanded,
                            onIconClick = { expanded = true }
                        )
                    },
                    colors = ExposedDropdownMenuDefaults.textFieldColors()
                )
                DropdownMenu(
                    modifier = Modifier
                        .width(with(LocalDensity.current) { textFieldWidth.width.toDp() }),
                    expanded = expanded,
                    onDismissRequest = {
                        expanded = false
                    },
                ) {
                    timeIntervals.forEachIndexed { index, seconds ->
                        DropdownMenuItem(
                            modifier = Modifier
                                .fillMaxWidth(),
                            onClick = {
                                expanded = false
                                selectedIndex = index
                                onPollIntervalModified()
                            },
                            ) {
                            Text(text = secondsToText(seconds))
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.padding(5.dp))

            Divider()

            Spacer(modifier = Modifier.padding(5.dp))

            ActionButton(
                text = stringResource(id = R.string.export_logs),
                onClick = onExportLogsClicked
            )

            Spacer(modifier = Modifier.padding(5.dp))
        }
    }
}

@Composable
fun secondsToText(seconds: ULong): String {
    val secondStr = stringResource(id = R.string.second)
    val secondsStr = stringResource(id = R.string.seconds)
    val minuteStr = stringResource(id = R.string.minute)
    val minutesStr = stringResource(id = R.string.minutes)
    return if(seconds == 1UL) {
        "$seconds $secondStr"
    } else if (seconds < 60UL) {
        "$seconds $secondsStr"
    } else if((seconds == 60UL) ) {
        "${(seconds / 60UL)} $minuteStr"
    } else {
        "${(seconds / 60UL)} $minutesStr"
    }
}