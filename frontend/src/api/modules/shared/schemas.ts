import { z } from 'zod'

/**
 * Standard backend error response schema.
 * Handles both { error: string, success: false } and generic { message: string }.
 */
export const ErrorPayloadSchema = z.object({
  error: z.string().optional(),
  message: z.string().optional(),
  success: z.boolean().optional(),
})

/**
 * Standard health status response schema.
 */
export const HealthStatusSchema = z.object({
  status: z.string(),
})
