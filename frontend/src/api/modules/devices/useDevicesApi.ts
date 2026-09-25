import { useEventSource, type UseEventSourceReturn } from '@vueuse/core'
import { useApiFetch } from '../../useApiFetch'
import {
  ControlRequestSchema,
  ControlResponseSchema,
  DeviceSnapshotSchema,
  DevicesListSchema,
  ProjectSnapshotSchema,
} from './schemas'
import type {
  ControlRequest,
  ControlResponse,
  DeviceSnapshot,
  DevicesList,
  ProjectSnapshot,
} from './types'

export type DevicesEventStream = UseEventSourceReturn<
  ['init', 'update'],
  ProjectSnapshot | null
>

export const useDevicesApi = () => {
  /**
   * Fetches the entire project snapshot including all devices.
   */
  async function getProject(): Promise<ProjectSnapshot> {
    const { data, error } = await useApiFetch('/project').get().json()

    if (error.value || !data.value) {
      throw error.value || new Error('Failed to fetch project snapshot')
    }

    return ProjectSnapshotSchema.parse(data.value)
  }

  /**
   * Fetches all device snapshots in the project.
   */
  async function getDevices(): Promise<DevicesList> {
    const { data, error } = await useApiFetch('/devices').get().json()

    if (error.value || !data.value) {
      throw error.value || new Error('Failed to fetch devices list')
    }

    return DevicesListSchema.parse(data.value)
  }

  /**
   * Fetches a specific device snapshot by its numeric ID.
   */
  async function getDevice(deviceId: number): Promise<DeviceSnapshot> {
    const { data, error } = await useApiFetch(`/devices/${deviceId}`).get().json()

    if (error.value || !data.value) {
      throw error.value || new Error(`Failed to fetch device ${deviceId}`)
    }

    return DeviceSnapshotSchema.parse(data.value)
  }

  /**
   * Sends a control command to an actuator on a device.
   * Performs defensive validation before dispatch and on response.
   */
  async function controlActuator(payload: ControlRequest): Promise<ControlResponse> {
    // 1. Validate outgoing payload
    const validatedPayload = ControlRequestSchema.parse(payload)

    // 2. Perform HTTP POST
    const { data, error } = await useApiFetch('/control').post(validatedPayload).json()

    // 3. Handle errors
    if (error.value || !data.value) {
      throw error.value || new Error('Failed to dispatch actuator command')
    }

    // 4. Validate incoming response
    return ControlResponseSchema.parse(data.value)
  }

  /**
   * Reactive Server-Sent Events (SSE) stream using VueUse's `useEventSource`.
   * Automatically parses and defensively validates inbound data with Zod `ProjectSnapshotSchema`.
   */
  function useEvents(
    url: string = import.meta.env.VITE_EVENTS_URL || '/api/events',
  ): DevicesEventStream {
    return useEventSource<['init', 'update'], ProjectSnapshot | null>(
      url,
      ['init', 'update'],
      {
        autoReconnect: {
          retries: -1,
          delay: 1500,
        },
        immediate: true,
        serializer: {
          read(raw?: string): ProjectSnapshot | null {
            if (!raw) return null
            try {
              const parsed = JSON.parse(raw)
              return ProjectSnapshotSchema.parse(parsed)
            } catch (err) {
              console.error('Failed to validate inbound SSE event with Zod:', err)
              return null
            }
          },
        },
      },
    )
  }

  return {
    getProject,
    getDevices,
    getDevice,
    controlActuator,
    useEvents,
  }
}
