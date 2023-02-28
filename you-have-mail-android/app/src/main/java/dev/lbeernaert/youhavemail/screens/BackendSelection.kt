package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import dev.lbeernaert.youhavemail.Backend
import dev.lbeernaert.youhavemail.Log
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.ServiceView

@Composable
fun BackendSelection(serviceView: ServiceView, navController: NavController) {
    Scaffold(topBar = {
        TopAppBar(title = {
            Text(text = stringResource(id = R.string.backend_title))
        },
            navigationIcon = if (navController.previousBackStackEntry != null) {
                {
                    IconButton(onClick = {
                        navController.popBackStack()
                    }) {
                        Icon(
                            imageVector = Icons.Filled.ArrowBack,
                            contentDescription = "Back"
                        )
                    }
                }
            } else {
                null
            }
        )
    },
        content = { _ ->
            BackendList(backends = serviceView.getBackend(), onClicked = {
                Log.e("INDEX=$it")
                navController.navigate(Routes.newLoginRoute(it))
            })
        }
    )
}

@Composable
fun BackendListItem(backend: Backend, index: Int, onClicked: (Int) -> Unit) {
    Row(
        modifier = Modifier
            .padding(10.dp)
            .fillMaxWidth()
            .clickable { onClicked(index) },
    ) {
        val name = backend.name()
        Column(
            verticalArrangement = Arrangement.Center,
            modifier = Modifier
                .size(40.dp)
                .background(MaterialTheme.colors.primary, MaterialTheme.shapes.small),
        ) {
            Text(
                modifier = Modifier.fillMaxWidth(),
                text = name.first().toString(),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.button,
                fontWeight = FontWeight.Bold,
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
fun BackendList(backends: List<Backend>, onClicked: (index: Int) -> Unit) {
    LazyColumn(contentPadding = PaddingValues(horizontal = 10.dp, vertical = 10.dp)) {
        itemsIndexed(backends) { index, backend ->
            BackendListItem(backend = backend, index = index, onClicked = onClicked)
        }
    }
}