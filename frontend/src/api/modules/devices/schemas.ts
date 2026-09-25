import { z } from 'zod'
import { ALL_TAGS } from './domain'

/**
 * Validated sensor and actuator tag enum schema.
 */
export const ActuatorTagSchema = z.enum(ALL_TAGS)

/**
 * Validated sensor telemetry value dictionary schema.
 */
export const SensorsRecordSchema = z.record(
  z.string(),
  z.union([z.string(), z.number(), z.boolean(), z.null()]),
)

/**
 * Device snapshot schema.
 * `sensors` is `null` when the device is offline or data cannot be fetched.
 */
export const DeviceSnapshotSchema = z.object({
  device_id: z.number().int(),
  name: z.string(),
  tag: z.string(),
  sensors: SensorsRecordSchema.nullable(),
})

/**
 * List of device snapshots schema.
 */
export const DevicesListSchema = z.array(DeviceSnapshotSchema)

/**
 * Project snapshot schema with all associated devices.
 */
export const ProjectSnapshotSchema = z.object({
  project_id: z.number().int(),
  name: z.string(),
  tag: z.string().nullable().optional(),
  devices: z.array(DeviceSnapshotSchema),
})

/**
 * Outgoing control request payload schema.
 */
export const ControlRequestSchema = z.object({
  device_id: z.number().int().optional(),
  tag: z.string().min(1),
  value: z.union([z.string(), z.number(), z.boolean()]),
})

/**
 * Successful control command response schema.
 */
export const ControlResponseSchema = z.object({
  success: z.boolean(),
  device_id: z.number().int(),
  tag: z.string(),
  value: z.union([z.string(), z.number(), z.boolean()]),
})
