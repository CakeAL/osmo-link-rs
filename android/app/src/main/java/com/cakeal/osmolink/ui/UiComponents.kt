package com.cakeal.osmolink.ui

import androidx.annotation.DrawableRes
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun RemoteTopBar(title: String, actions: @Composable RowScope.() -> Unit) {
    TopAppBar(
        modifier = Modifier.widthIn(max = 560.dp),
        title = { Text(title) },
        actions = actions,
        windowInsets = WindowInsets(0, 0, 0, 0),
    )
}

@Composable
internal fun RemoteCard(content: @Composable ColumnScope.() -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.extraLarge,
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
        content = content,
    )
}

@Composable
internal fun SectionTitle(title: String, @DrawableRes icon: Int, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier.semantics { heading() },
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        UiIcon(icon, Modifier.size(18.dp))
        Text(title, style = MaterialTheme.typography.titleMedium)
    }
}

@Composable
internal fun StatusPill(
    text: String,
    @DrawableRes icon: Int? = null,
    contentColor: Color = LocalContentColor.current,
    color: Color = contentColor.copy(alpha = 0.10f),
) {
    Surface(shape = CircleShape, color = color, contentColor = contentColor) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (icon != null) UiIcon(icon, Modifier.size(14.dp))
            Text(text, style = MaterialTheme.typography.labelMedium)
        }
    }
}

@Composable
internal fun DetailRow(label: String, value: String, valueColor: Color = MaterialTheme.colorScheme.onSurface) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.weight(1f),
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = valueColor,
            textAlign = TextAlign.End,
            modifier = Modifier.weight(2f),
        )
    }
}
