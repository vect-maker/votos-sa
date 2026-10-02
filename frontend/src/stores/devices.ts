import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { useDevicesApi } from '@/api/modules/devices/useDevicesApi'
import { TAG_BRIGHTNESS } from '@/api/modules/devices/domain'
import type {
  ControlRequest,
  ControlResponse,
  DeviceSnapshot,
  ProjectSnapshot,
} from '@/api/modules/devices/types'

export interface LuxHistoryPoint {
  timestamp: Date
  time: string
  lux: number
}

export const useDevicesStore = defineStore('devices', () => {
  const devicesApi = useDevicesApi()

  // State
  const manualProject = ref<ProjectSnapshot | null>(null)
  const isFetching = ref(false)
  const error = ref<string | null>(null)
  const luxHistory = ref<Record<number, LuxHistoryPoint[]>>({})

  // Real-time EventSource stream via VueUse
  const {
    data: streamData,
    status: streamStatus,
    error: streamError,
    close: stopLiveSync,
    open: startLiveSync,
  } = devicesApi.useEvents()

  // Computed / Getters
  const isConnected = computed(() => streamStatus.value === 'OPEN')

  const project = computed<ProjectSnapshot | null>(() => {
    return streamData.value || manualProject.value
  })

  const devices = computed<DeviceSnapshot[]>(() => project.value?.devices || [])

  const primaryDevice = computed<DeviceSnapshot | null>(() => devices.value[0] || null)

  const isOnline = computed<boolean>(() => primaryDevice.value?.sensors != null)

  const sensors = computed<Record<string, string | number | boolean | null>>(() => {
    return primaryDevice.value?.sensors || {}
  })

  // Watch devices for real-time telemetry updates and track rolling lux history
  watch(
    () => devices.value,
    (devs) => {
      const now = new Date()
      const timeStr = now.toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      })

      for (const d of devs) {
        if (!d.sensors) continue
        const raw = d.sensors[TAG_BRIGHTNESS]
        if (raw == null) continue

        const val = typeof raw === 'number' ? Math.round(raw) : parseFloat(String(raw))
        if (isNaN(val)) continue

        let history = luxHistory.value[d.device_id]
        if (!history) {
          history = []
          luxHistory.value[d.device_id] = history
        }

        const lastPoint = history[history.length - 1]

        // Record point if it's the first one, or at least 1s elapsed
        if (!lastPoint || now.getTime() - lastPoint.timestamp.getTime() >= 1000) {
          history.push({
            timestamp: now,
            time: timeStr,
            lux: Math.round(val),
          })

          // Maintain rolling window of 30 points
          if (history.length > 30) {
            history.shift()
          }
        }
      }
    },
    { deep: true, immediate: true },
  )

  function getDeviceLuxHistory(deviceId?: number): LuxHistoryPoint[] {
    const targetId = deviceId ?? primaryDevice.value?.device_id
    if (!targetId || !luxHistory.value[targetId]) {
      return []
    }
    return luxHistory.value[targetId]
  }

  const primaryLuxHistory = computed<LuxHistoryPoint[]>(() => {
    return getDeviceLuxHistory()
  })

  // Actions
  async function fetchProject() {
    isFetching.value = true
    error.value = null
    try {
      manualProject.value = await devicesApi.getProject()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      console.error('Failed to load project snapshot:', err)
    } finally {
      isFetching.value = false
    }
  }

  async function sendControl(payload: ControlRequest): Promise<ControlResponse> {
    error.value = null
    try {
      const res = await devicesApi.controlActuator(payload)
      // Optimistically update device sensor state in active snapshot
      const currentProj = streamData.value || manualProject.value
      if (currentProj) {
        const targetId = payload.device_id ?? currentProj.devices[0]?.device_id
        const targetDev = currentProj.devices.find((d) => d.device_id === targetId)
        if (targetDev) {
          if (!targetDev.sensors) {
            targetDev.sensors = {}
          }
          targetDev.sensors[payload.tag] = res.value
        }
      }
      return res
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err)
      error.value = msg
      throw err
    }
  }

  function getDeviceById(id: number | string): DeviceSnapshot | null {
    const numericId = typeof id === 'string' ? parseInt(id, 10) : id
    return devices.value.find((d) => d.device_id === numericId) || null
  }

  return {
    // State & Stream
    project,
    isFetching,
    error,
    isConnected,
    streamStatus,
    streamError,
    // Getters
    devices,
    primaryDevice,
    isOnline,
    sensors,
    getDeviceById,
    // Telemetry History
    luxHistory,
    primaryLuxHistory,
    getDeviceLuxHistory,
    // Actions
    fetchProject,
    startLiveSync,
    stopLiveSync,
    sendControl,
  }
})
