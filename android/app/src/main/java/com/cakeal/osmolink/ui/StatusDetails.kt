package com.cakeal.osmolink.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.cakeal.osmolink.R
import com.cakeal.osmolink.data.UiState

@Composable
internal fun StatusDetails(state: UiState) {
    val status = state.status
    RemoteCard {
        Column(
            modifier = Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            SectionTitle("相机详情", R.drawable.ic_ui_info)
            if (state.deviceName.isNotBlank() && state.modelName.isNotBlank() && state.deviceName != state.modelName) {
                DetailRow("设备名称", state.deviceName)
            }
            if (status.hasData) {
                DetailRow("可用容量", "%.1f GB".format(status.remainingMb / 1024.0))
                DetailRow("防抖", status.eis.ifBlank { "—" })
                if (status.temperatureCode >= 2) {
                    DetailRow("温度", temperatureLabel(status), valueColor = MaterialTheme.colorScheme.error)
                }
            }
            state.version?.let { DetailRow("固件版本", it) }
            state.modeInfo?.let { DetailRow("模式参数", it) }
            if (!status.hasData && state.version == null && state.modeInfo == null) {
                Text(
                    "连接后自动读取相机详情。",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
