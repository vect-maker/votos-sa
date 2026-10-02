<script setup lang="ts">
import { ref, computed } from 'vue'
import { useDevicesStore } from '@/stores/devices'
import { Sun } from '@lucide/vue'
import { Line } from 'vue-chartjs'
import {
  Chart as ChartJS,
  Title,
  Tooltip,
  Legend,
  LineElement,
  LinearScale,
  PointElement,
  CategoryScale,
  Filler,
  type ChartOptions,
  type ChartData,
} from 'chart.js'

ChartJS.register(
  Title,
  Tooltip,
  Legend,
  LineElement,
  LinearScale,
  PointElement,
  CategoryScale,
  Filler,
)

interface Props {
  deviceId?: number | string
}

const props = defineProps<Props>()
const store = useDevicesStore()

// If there are multiple devices and no explicit deviceId prop, allow selecting which device's lux to inspect
const selectedDeviceId = ref<number | null>(null)

const devicesWithLux = computed(() => {
  return store.devices.filter((d) => d.sensors !== null)
})

const activeDeviceId = computed<number | null>(() => {
  if (props.deviceId != null) {
    const parsed = typeof props.deviceId === 'string' ? parseInt(props.deviceId, 10) : props.deviceId
    return isNaN(parsed) ? null : parsed
  }
  if (selectedDeviceId.value !== null) {
    return selectedDeviceId.value
  }
  return store.primaryDevice?.device_id ?? null
})

const activeDevice = computed(() => {
  if (!activeDeviceId.value) return store.primaryDevice
  return store.getDeviceById(activeDeviceId.value)
})

const history = computed(() => {
  if (!activeDeviceId.value) return []
  return store.getDeviceLuxHistory(activeDeviceId.value)
})

const currentLux = computed<number | null>(() => {
  if (history.value.length > 0) {
    const last = history.value[history.value.length - 1]
    return last ? last.lux : null
  }
  const raw = activeDevice.value?.sensors?.brightness
  if (typeof raw === 'number') return Math.round(raw)
  if (typeof raw === 'string') {
    const parsed = parseFloat(raw)
    return isNaN(parsed) ? null : Math.round(parsed)
  }
  return null
})

const minLux = computed<number | null>(() => {
  if (history.value.length === 0) return currentLux.value
  return Math.min(...history.value.map((p) => p.lux))
})

const maxLux = computed<number | null>(() => {
  if (history.value.length === 0) return currentLux.value
  return Math.max(...history.value.map((p) => p.lux))
})

const avgLux = computed<number | null>(() => {
  if (history.value.length === 0) return currentLux.value
  const sum = history.value.reduce((acc, p) => acc + p.lux, 0)
  return Math.round(sum / history.value.length)
})

// Chart.js data configuration
const chartData = computed<ChartData<'line'>>(() => {
  const labels = history.value.map((p) => p.time)
  const data = history.value.map((p) => p.lux)

  return {
    labels,
    datasets: [
      {
        label: 'Ambient Light (lx)',
        data,
        borderColor: '#f59e0b',
        backgroundColor: 'rgba(245, 158, 11, 0.12)',
        borderWidth: 2,
        tension: 0.35,
        fill: true,
        pointBackgroundColor: '#f59e0b',
        pointBorderColor: '#ffffff',
        pointBorderWidth: 1.5,
        pointRadius: (ctx) => {
          return ctx.dataIndex === data.length - 1 ? 5 : 2
        },
        pointHoverRadius: 6,
      },
    ],
  }
})

// Chart.js options
const chartOptions = computed<ChartOptions<'line'>>(() => ({
  responsive: true,
  maintainAspectRatio: false,
  animation: {
    duration: 350,
    easing: 'easeOutQuart',
  },
  interaction: {
    mode: 'index',
    intersect: false,
  },
  plugins: {
    legend: {
      display: false,
    },
    tooltip: {
      backgroundColor: 'rgba(24, 24, 27, 0.92)',
      titleColor: '#ffffff',
      bodyColor: '#fbbf24',
      padding: 10,
      cornerRadius: 8,
      callbacks: {
        label: (context) => `Ambient Light: ${context.parsed.y} lx`,
      },
    },
  },
  scales: {
    y: {
      beginAtZero: true,
      suggestedMax: maxLux.value ? Math.ceil(maxLux.value * 1.15) : 300,
      grid: {
        color: 'rgba(156, 163, 175, 0.12)',
      },
      ticks: {
        callback: (val) => `${val} lx`,
        font: {
          size: 10,
          family: 'monospace',
        },
        maxTicksLimit: 5,
      },
    },
    x: {
      grid: {
        display: false,
      },
      ticks: {
        font: {
          size: 10,
          family: 'monospace',
        },
        maxTicksLimit: 6,
      },
    },
  },
}))
</script>

<template>
  <div class="card bg-base-100 border border-base-300 shadow-sm p-5 flex flex-col gap-4">
    <!-- Header with Title, Stats & Device Selector -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-base-200 pb-4">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-warning/10 text-warning flex items-center justify-center shrink-0">
          <Sun class="w-5 h-5" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h2 class="text-base font-bold tracking-tight">Ambient Light Telemetry</h2>
            <div
              v-if="store.isConnected && currentLux !== null"
              class="flex items-center gap-1.5 text-[11px] font-semibold text-success bg-success/10 px-2 py-0.5 rounded-full"
            >
              <span class="w-1.5 h-1.5 rounded-full bg-success animate-pulse"></span>
              <span>Live</span>
            </div>
          </div>
          <p class="text-xs text-base-content/60 mt-0.5 font-mono">
            {{ activeDevice?.name || 'Device' }} • Real-time lux readings
          </p>
        </div>
      </div>

      <!-- Quick Metrics Summary -->
      <div class="flex items-center gap-4 flex-wrap">
        <div v-if="currentLux !== null" class="flex items-baseline gap-1.5 bg-base-200/70 px-3 py-1.5 rounded-box border border-base-300/50">
          <span class="text-xs text-base-content/60 font-medium">Current:</span>
          <span class="text-base font-bold text-warning font-mono">{{ currentLux }} lx</span>
        </div>

        <div v-if="history.length > 1" class="hidden md:flex items-center gap-3 text-xs font-mono text-base-content/60">
          <span>Min: <strong class="text-base-content">{{ minLux }} lx</strong></span>
          <span>•</span>
          <span>Avg: <strong class="text-base-content">{{ avgLux }} lx</strong></span>
          <span>•</span>
          <span>Peak: <strong class="text-base-content">{{ maxLux }} lx</strong></span>
        </div>

        <!-- Device Selector (if multiple devices available and no explicit deviceId provided) -->
        <select
          v-if="!props.deviceId && devicesWithLux.length > 1"
          v-model="selectedDeviceId"
          class="select select-xs select-bordered font-mono text-xs"
        >
          <option :value="null">Primary Device</option>
          <option
            v-for="d in devicesWithLux"
            :key="d.device_id"
            :value="d.device_id"
          >
            {{ d.name }} (#{{ d.device_id }})
          </option>
        </select>
      </div>
    </div>

    <!-- Graph Canvas / Empty State -->
    <div class="relative w-full h-56 sm:h-64">
      <Line
        v-if="history.length > 0"
        :data="chartData"
        :options="chartOptions"
      />

      <!-- Empty State if no lux data received yet -->
      <div
        v-else
        class="h-full flex flex-col items-center justify-center gap-2 p-6 text-center bg-base-200/30 rounded-box border border-dashed border-base-300"
      >
        <div class="w-10 h-10 rounded-full bg-base-200 flex items-center justify-center text-base-content/40">
          <Sun class="w-5 h-5 text-warning/50 animate-pulse" />
        </div>
        <p class="text-xs font-medium text-base-content/70">
          {{
            store.isOnline
              ? 'Waiting for ambient light readings from device...'
              : 'Device is offline. Connect the device to view real-time lux history.'
          }}
        </p>
        <span class="text-[10px] text-base-content/50 font-mono">
          Updates stream automatically via Server-Sent Events
        </span>
      </div>
    </div>
  </div>
</template>
