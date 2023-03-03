package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material.CircularProgressIndicator
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.R

@Composable
fun BackgroundTask(text: String) {
    Column(
        modifier = Modifier
            .padding(10.dp)
            .fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,

        ) {
        CircularProgressIndicator()
        Spacer(modifier = Modifier.padding(10.dp))
        Text(text = text)
    }
}