package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.NavController
import dev.lbeernaert.youhavemail.Backend
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.app.State

@Composable
fun BackendSelection(state: State, navController: NavController) {
    Scaffold(topBar = {
        TopAppBar(title = {
            Text(text = stringResource(id = R.string.backend_title))
        },
            navigationIcon =
            {
                IconButton(onClick = {
                    navController.popBackStack()
                }) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                        contentDescription = "Back"
                    )
                }
            }
        )
    },
        content = { _ ->
            BackendList(backends = state.backends(), onClicked = {
                navController.navigate(Routes.newProxyLoginRoute(it))
            })
        }
    )
}

@Composable
fun BackendListItem(backend: Backend, onClicked: (String) -> Unit) {
    Row(
        modifier = Modifier
            .padding(10.dp)
            .fillMaxWidth()
            .clickable { onClicked(backend.name()) },
    ) {
        val name = backend.name()
        Column(
            verticalArrangement = Arrangement.Center,
            modifier = Modifier
                .size(60.dp)
                .background(MaterialTheme.colors.primary, MaterialTheme.shapes.large),
        ) {
            Text(
                modifier = Modifier.fillMaxWidth(),
                text = name.first().toString(),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.button,
                fontWeight = FontWeight.Bold,
                fontSize = 30.sp
            )
        }
        Spacer(modifier = Modifier.width(10.dp))
        Column(modifier = Modifier.fillMaxWidth()) {
            Text(
                text = name,
                style = MaterialTheme.typography.subtitle1,
                fontWeight = FontWeight.Bold
            )
            Text(text = backend.description(), style = MaterialTheme.typography.body2)
        }
    }
}

@Composable
fun BackendList(backends: List<Backend>, onClicked: (index: String) -> Unit) {
    LazyColumn(contentPadding = PaddingValues(horizontal = 10.dp, vertical = 10.dp)) {
        itemsIndexed(backends) { _, backend ->
            BackendListItem(backend = backend, onClicked = onClicked)
        }
    }
}