import type { z } from 'zod'
import type {
  ActuatorTagSchema,
  ControlRequestSchema,
  ControlResponseSchema,
  DeviceSnapshotSchema,
  DevicesListSchema,
  ProjectSnapshotSchema,
  SensorsRecordSchema,
} from './schemas'

export type ActuatorTag = z.infer<typeof ActuatorTagSchema>
export type SensorsRecord = z.infer<typeof SensorsRecordSchema>
export type DeviceSnapshot = z.infer<typeof DeviceSnapshotSchema>
export type DevicesList = z.infer<typeof DevicesListSchema>
export type ProjectSnapshot = z.infer<typeof ProjectSnapshotSchema>
export type ControlRequest = z.infer<typeof ControlRequestSchema>
export type ControlResponse = z.infer<typeof ControlResponseSchema>
