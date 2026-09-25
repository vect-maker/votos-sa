<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import type { DeviceSnapshot } from '@/api/modules/devices/types'
import { TAG_BRIGHTNESS } from '@/api/modules/devices/domain'
import {
  Cpu,
  Wifi,
  WifiOff,
  ChevronRight,
  Sun,
  Activity,
  Layers,
} from '@lucide/vue'

interface Props {
  device: DeviceSnapshot
}

const props = defineProps<Props>()

const isOnline = computed(() => props.device.sensors !== null)

const sensorCount = computed(() => {
  if (!props.device.sensors) return 0
  return Object.keys(props.device.sensors).length
})

const brightnessValue = computed<number | null>(() => {
  if (!props.device.sensors) return null
  const val = props.device.sensors[TAG_BRIGHTNESS]
  if (typeof val === 'number') return Math.round(val)
  if (typeof val === 'string') {
    const parsed = parseFloat(val)
    return isNaN(parsed) ? null : Math.round(parsed)
  }
  return null
})
</script>

<template>
  <div
    class="card bg-base-100 border border-base-300 shadow-sm hover:shadow-lg transition-all duration-200 flex flex-col justify-between"
    :class="{ 'ring-1 ring-primary/20': isOnline }"
  >
    <div class="card-body p-5 flex flex-col gap-4">
      <!-- Header: Icon & Online Status Badge -->
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-2.5">
          <div
            class="w-10 h-10 rounded-xl flex items-center justify-center transition-colors"
            :class="isOnline ? 'bg-primary/10 text-primary' : 'bg-base-200 text-base-content/50'"
          >
            <Cpu class="w-5 h-5" />
          </div>
          <div>
            <h2 class="card-title text-base font-bold tracking-tight leading-snug">
              {{ device.name }}
            </h2>
            <div class="flex items-center gap-1.5 text-xs text-base-content/60 font-mono mt-0.5">
              <span>#{{ device.device_id }}</span>
              <span>•</span>
              <span class="badge badge-xs badge-ghost font-mono">{{ device.tag }}</span>
            </div>
          </div>
        </div>

        <!-- Online / Offline Badge -->
        <div
          class="badge gap-1.5 py-2.5 px-3 text-xs font-semibold shrink-0"
          :class="isOnline ? 'badge-success text-success-content' : 'badge-ghost text-base-content/60'"
        >
          <span
            v-if="isOnline"
            class="w-2 h-2 rounded-full bg-success-content animate-pulse"
          ></span>
          <component :is="isOnline ? Wifi : WifiOff" class="w-3.5 h-3.5" />
          {{ isOnline ? 'Online' : 'Offline' }}
        </div>
      </div>

      <!-- Quick Telemetry Preview -->
      <div class="bg-base-200/60 rounded-box p-3 border border-base-300/50 text-xs">
        <div v-if="isOnline" class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Activity class="w-4 h-4 text-primary" />
            <span class="text-base-content/80 font-medium">
              {{ sensorCount }} {{ sensorCount === 1 ? 'Sensor' : 'Sensors' }} Active
            </span>
          </div>
          <div v-if="brightnessValue !== null" class="flex items-center gap-1 text-warning font-semibold">
            <Sun class="w-3.5 h-3.5" />
            <span>{{ brightnessValue }} lx</span>
          </div>
        </div>
        <div v-else class="flex items-center gap-2 text-base-content/60">
          <Layers class="w-4 h-4" />
          <span>Device powered off / telemetry offline</span>
        </div>
      </div>

      <!-- Card Action Link -->
      <div class="card-actions pt-2 mt-auto">
        <RouterLink
          :to="{ name: 'device-detail', params: { id: device.device_id } }"
          class="btn btn-sm btn-primary w-full gap-2 group"
        >
          <span>Open Controls & Telemetry</span>
          <ChevronRight class="w-4 h-4 transition-transform group-hover:translate-x-1" />
        </RouterLink>
      </div>
    </div>
  </div>
</template>
