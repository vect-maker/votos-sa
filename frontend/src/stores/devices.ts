import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useDevicesApi } from '@/api/modules/devices/useDevicesApi'
import type {
  ControlRequest,
  ControlResponse,
  DeviceSnapshot,
  ProjectSnapshot,
} from '@/api/modules/devices/types'

export const useDevicesStore = defineStore('devices', () => {
  const devicesApi = useDevicesApi()

  // State
  const manualProject = ref<ProjectSnapshot | null>(null)
  const isFetching = ref(false)
  const error = ref<string | null>(null)

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
    // Actions
    fetchProject,
    startLiveSync,
    stopLiveSync,
    sendControl,
  }
})
