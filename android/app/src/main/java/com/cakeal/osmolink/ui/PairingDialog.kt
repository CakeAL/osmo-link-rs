package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.data.PairingInfo

@Composable
internal fun PairingDialog(pairing: PairingInfo, onCancel: () -> Unit) {
    AlertDialog(
        onDismissRequest = {},
        containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
        titleContentColor = MaterialTheme.colorScheme.onSurface,
        textContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
        title = { Text(if (pairing.firstTime) "首次配对" else "重新连接") },
        text = {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("请在相机屏幕上确认配对码", style = MaterialTheme.typography.bodySmall)
                Text(
                    text = "%04d".format(pairing.code),
                    style = MaterialTheme.typography.displaySmall.mono(),
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(pairing.device, style = MaterialTheme.typography.bodySmall)
            }
        },
        confirmButton = {},
        dismissButton = {
            TextButton(onClick = onCancel) {
                Text("取消", style = MaterialTheme.typography.labelLarge)
            }
        },
    )
}
