import type { z } from 'zod'
import type { ErrorPayloadSchema, HealthStatusSchema } from './schemas'

export type ErrorPayload = z.infer<typeof ErrorPayloadSchema>
export type HealthStatus = z.infer<typeof HealthStatusSchema>
