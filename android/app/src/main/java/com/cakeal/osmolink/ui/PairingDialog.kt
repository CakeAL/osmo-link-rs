package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.PairingInfo

@Composable
internal fun PairingDialog(pairing: PairingInfo, onCancel: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    AlertDialog(
        onDismissRequest = {},
        icon = {
            Surface(
                shape = CircleShape,
                color = colors.primaryContainer,
                contentColor = colors.onPrimaryContainer,
            ) {
                Box(Modifier.size(44.dp), contentAlignment = Alignment.Center) {
                    UiIcon(R.drawable.ic_ui_camera)
                }
            }
        },
        title = { Text(if (pairing.firstTime) "首次配对" else "重新连接") },
        text = {
            Column(
                modifier = Modifier.fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("请在 ${pairing.device} 上确认配对码", textAlign = TextAlign.Center)
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    shape = MaterialTheme.shapes.large,
                    color = colors.surfaceContainerHighest,
                    contentColor = colors.onSurface,
                ) {
                    Text(
                        text = "%04d".format(pairing.code),
                        modifier = Modifier.padding(12.dp),
                        style = MaterialTheme.typography.headlineLarge.mono(),
                        textAlign = TextAlign.Center,
                    )
                }
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                    Text("正在等待相机", style = MaterialTheme.typography.bodySmall)
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onCancel) { Text("取消") } },
    )
}
