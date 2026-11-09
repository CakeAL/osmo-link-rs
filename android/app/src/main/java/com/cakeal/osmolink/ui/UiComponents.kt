package com.cakeal.osmolink.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.clickable
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import com.cakeal.osmolink.R

@Composable
internal fun UiIcon(@DrawableRes icon: Int, modifier: Modifier = Modifier) {
    Icon(painterResource(icon), contentDescription = null, modifier = modifier)
}

@Composable
internal fun DisclosureRow(
    title: String,
    @DrawableRes icon: Int,
    expanded: Boolean,
    onToggle: () -> Unit,
    supportingContent: (@Composable () -> Unit)? = null,
) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = supportingContent,
        leadingContent = { UiIcon(icon) },
        trailingContent = {
            UiIcon(R.drawable.ic_ui_chevron, Modifier.rotate(if (expanded) 180f else 0f))
        },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        modifier = Modifier
            .semantics { stateDescription = if (expanded) "已展开" else "已折叠" }
            .clickable(role = Role.Button, onClick = onToggle),
    )
}

@Composable
internal fun SettingsButton(onClick: () -> Unit, enabled: Boolean = true) {
    IconButton(onClick = onClick, enabled = enabled) {
        Icon(painterResource(R.drawable.ic_ui_gear), contentDescription = "设置")
    }
}
