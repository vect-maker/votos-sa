import { createFetch } from '@vueuse/core'
import { ErrorPayloadSchema } from './modules/shared/schemas'

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '/api'

export const useApiFetch = createFetch({
  baseUrl: API_BASE_URL,
  options: {
    updateDataOnError: true,

    // 1. Request Interceptor
    beforeFetch({ options }) {
      const headers = new Headers(options.headers)
      if (!headers.has('Content-Type')) {
        headers.set('Content-Type', 'application/json')
      }
      options.headers = headers
      return { options }
    },

    // 2. Response & Error Interceptor
    async onFetchError(ctx) {
      const { data } = ctx

      if (data) {
        if (typeof data === 'string' && data.trim()) {
          ctx.error = new Error(data)
        } else if (typeof data === 'object' && data !== null) {
          const parsed = ErrorPayloadSchema.safeParse(data)
          if (parsed.success) {
            const msg = parsed.data.error || parsed.data.message
            if (msg) {
              ctx.error = new Error(msg)
            }
          }
        }
      }

      return ctx
    },
  },
  fetchOptions: {
    credentials: 'same-origin',
  },
})
