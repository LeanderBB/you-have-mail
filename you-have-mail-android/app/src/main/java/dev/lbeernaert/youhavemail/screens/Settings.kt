package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
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


@OptIn(ExperimentalMaterial3Api::class)
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
    ) { padding, runTask ->

        val pollIntervalValue by state.getPollInterval().collectAsState()
        val updatingPollIntervalLabel = stringResource(id = R.string.update_poll_interval)

        Column(
            modifier = Modifier
                .padding(padding)
                .padding(20.dp)
                .fillMaxSize()
        ) {
            val timeIntervals =
                listOf(
                    30UL, 60UL, 120UL, 180UL, 300UL, 600UL, 900UL, 1200UL, 1800UL, 3600UL,
                    7200UL, 14400UL, 28800UL, 43200UL, 57600UL, 86400UL
                )
            var expanded by remember { mutableStateOf(false) }
            var selectedIndex by remember { mutableIntStateOf(0) }
            // textFieldWidth is used to assign to DropDownMenu the same width as TextField
            var textFieldWidth by remember { mutableStateOf(Size.Zero) }

            val onPollIntervalModified: () -> Unit = {
                runTask(updatingPollIntervalLabel) {
                    onPollIntervalUpdate(timeIntervals[selectedIndex])
                }
            }

            Text(
                text = stringResource(id = R.string.poll_interval),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.titleSmall,
                modifier = Modifier.fillMaxWidth()
            )
            Spacer(modifier = Modifier.padding(5.dp))
            Text(
                text = stringResource(id = R.string.poll_interval_desc),
                style = MaterialTheme.typography.titleMedium
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
                        .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable)
                        .fillMaxWidth()
                        .onGloballyPositioned { coordinates ->
                            textFieldWidth = coordinates.size.toSize()
                        },
                    readOnly = true,
                    value = secondsToText(pollIntervalValue),
                    onValueChange = { },
                    trailingIcon = {
                        ExposedDropdownMenuDefaults.TrailingIcon(
                            expanded = expanded
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
                    }
                ) {
                    timeIntervals.forEachIndexed { index, seconds ->
                        DropdownMenuItem(
                            text = { Text(text = secondsToText(seconds)) },
                            onClick = {
                                expanded = false
                                selectedIndex = index
                                onPollIntervalModified()
                            },
                            modifier = Modifier.fillMaxWidth()
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.padding(5.dp))

            HorizontalDivider()

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
    val hourStr = stringResource(id = R.string.hour)
    val hoursStr = stringResource(id = R.string.hours)
    return if (seconds == 1UL) {
        "1 $secondStr"
    } else if (seconds < 60UL) {
        "$seconds $secondsStr"
    } else if ((seconds == 60UL)) {
        "1 $minuteStr"
    } else if (seconds < 3600UL) {
        "${(seconds / 60UL)} $minutesStr"
    } else if (seconds == 3600UL) {
        "1 $hourStr"
    } else {
        "${(seconds / 3600UL)} $hoursStr"
    }
}