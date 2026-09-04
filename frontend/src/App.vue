<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

function getWsUrl(path: string = '/ws'): string {
  const baseUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:3000'
  const wsBase = baseUrl.replace(/^http(s?):\/\//, 'ws$1://').replace(/\/+$/, '')
  const cleanPath = path.startsWith('/') ? path : `/${path}`
  return `${wsBase}${cleanPath}`
}

const wsUrl = getWsUrl('/ws')
const status = ref<'CONNECTING' | 'OPEN' | 'CLOSED'>('CONNECTING')
const inputMessage = ref('')
const messages = ref<string[]>([])

let socket: WebSocket | null = null

function connect() {
  socket = new WebSocket(wsUrl)
  status.value = 'CONNECTING'

  socket.onopen = () => {
    status.value = 'OPEN'
  }

  socket.onmessage = (event: MessageEvent) => {
    messages.value.push(event.data)
  }

  socket.onclose = () => {
    status.value = 'CLOSED'
  }

  socket.onerror = () => {
    socket?.close()
  }
}

function sendMessage() {
  if (!inputMessage.value.trim() || socket?.readyState !== WebSocket.OPEN) return
  socket.send(inputMessage.value)
  inputMessage.value = ''
}

onMounted(() => {
  connect()
})

onUnmounted(() => {
  socket?.close()
})
</script>

<template>
  <main class="min-h-screen bg-base-200 p-8 flex flex-col items-center">
    <div class="card w-full max-w-lg bg-base-100 shadow-xl p-6 flex flex-col gap-4">
      <div class="flex items-center justify-between">
        <h1 class="text-xl font-bold">WebSocket Broadcast Test</h1>
        <span
          class="badge"
          :class="{
            'badge-success': status === 'OPEN',
            'badge-warning': status === 'CONNECTING',
            'badge-error': status === 'CLOSED',
          }"
        >
          {{ status }}
        </span>
      </div>

      <div class="text-xs text-base-content/60 font-mono">
        Derived Target: {{ wsUrl }}
      </div>

      <div class="h-64 overflow-y-auto border border-base-300 rounded-box p-3 bg-base-200 flex flex-col gap-2">
        <div v-if="messages.length === 0" class="text-center text-sm text-base-content/50 my-auto">
          No broadcast messages yet.
        </div>
        <div
          v-for="(msg, idx) in messages"
          :key="idx"
          class="p-2 bg-base-100 rounded text-sm break-words shadow-xs"
        >
          {{ msg }}
        </div>
      </div>

      <form @submit.prevent="sendMessage" class="flex gap-2">
        <input
          v-model="inputMessage"
          type="text"
          placeholder="Type a broadcast message..."
          class="input input-bordered flex-1"
          :disabled="status !== 'OPEN'"
        />
        <button
          type="submit"
          class="btn btn-primary"
          :disabled="status !== 'OPEN' || !inputMessage.trim()"
        >
          Send
        </button>
      </form>
    </div>
  </main>
</template>
