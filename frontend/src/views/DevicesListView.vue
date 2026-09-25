<script setup lang="ts">
import { ref, computed } from 'vue'
import { useDevicesStore } from '@/stores/devices'
import DeviceCard from '@/components/devices/DeviceCard.vue'
import {
  Layers,
  Wifi,
  WifiOff,
  Search,
  RefreshCw,
  FolderGit2,
} from '@lucide/vue'

const store = useDevicesStore()
const searchQuery = ref('')

const totalCount = computed(() => store.devices.length)

const onlineCount = computed(() => {
  return store.devices.filter((d) => d.sensors !== null).length
})

const offlineCount = computed(() => {
  return store.devices.filter((d) => d.sensors === null).length
})

const filteredDevices = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) return store.devices

  return store.devices.filter((device) => {
    return (
      device.name.toLowerCase().includes(query) ||
      device.tag.toLowerCase().includes(query) ||
      device.device_id.toString().includes(query)
    )
  })
})
</script>

<template>
  <div class="flex flex-col gap-6">
    <!-- Overview Metric Cards -->
    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
      <div class="stat bg-base-100 rounded-box border border-base-300 shadow-sm p-4">
        <div class="stat-figure text-primary">
          <Layers class="w-8 h-8 opacity-80" />
        </div>
        <div class="stat-title text-xs font-semibold uppercase tracking-wider text-base-content/60">
          Total Devices
        </div>
        <div class="stat-value text-2xl font-bold">{{ totalCount }}</div>
        <div class="stat-desc text-xs mt-1 text-base-content/60">Registered in project</div>
      </div>

      <div class="stat bg-base-100 rounded-box border border-base-300 shadow-sm p-4">
        <div class="stat-figure text-success">
          <Wifi class="w-8 h-8 opacity-80" />
        </div>
        <div class="stat-title text-xs font-semibold uppercase tracking-wider text-base-content/60">
          Online
        </div>
        <div class="stat-value text-2xl font-bold text-success">{{ onlineCount }}</div>
        <div class="stat-desc text-xs mt-1 text-base-content/60">Actively reporting telemetry</div>
      </div>

      <div class="stat bg-base-100 rounded-box border border-base-300 shadow-sm p-4">
        <div class="stat-figure text-base-content/40">
          <WifiOff class="w-8 h-8 opacity-80" />
        </div>
        <div class="stat-title text-xs font-semibold uppercase tracking-wider text-base-content/60">
          Offline
        </div>
        <div class="stat-value text-2xl font-bold text-base-content/60">{{ offlineCount }}</div>
        <div class="stat-desc text-xs mt-1 text-base-content/60">Sensors null / sleeping</div>
      </div>
    </div>

    <!-- Section Bar: Filter & Actions -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 bg-base-100 p-4 rounded-box border border-base-300 shadow-sm">
      <div class="relative flex-1 max-w-sm">
        <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-base-content/50" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Filter devices by name, tag, or ID..."
          class="input input-sm input-bordered w-full pl-9 pr-4 text-xs"
        />
      </div>

      <div class="flex items-center gap-2 text-xs text-base-content/70">
        <FolderGit2 class="w-4 h-4 text-primary" />
        <span>Project: <strong class="text-base-content">{{ store.project?.name || 'Loading...' }}</strong></span>
        <span v-if="store.project" class="badge badge-sm badge-ghost font-mono">#{{ store.project.project_id }}</span>
      </div>
    </div>

    <!-- Devices Grid -->
    <div v-if="filteredDevices.length > 0" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      <DeviceCard
        v-for="device in filteredDevices"
        :key="device.device_id"
        :device="device"
      />
    </div>

    <!-- Loading Skeleton State -->
    <div
      v-else-if="store.isFetching && store.devices.length === 0"
      class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5"
    >
      <div
        v-for="i in 3"
        :key="i"
        class="card bg-base-100 border border-base-300 shadow-sm p-5 animate-pulse flex flex-col gap-4"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 bg-base-300 rounded-xl"></div>
            <div class="space-y-2">
              <div class="h-4 bg-base-300 rounded w-28"></div>
              <div class="h-3 bg-base-300 rounded w-16"></div>
            </div>
          </div>
          <div class="h-6 bg-base-300 rounded-full w-16"></div>
        </div>
        <div class="h-14 bg-base-300/60 rounded-box"></div>
        <div class="h-8 bg-base-300 rounded-lg w-full mt-2"></div>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-else
      class="card bg-base-100 border border-base-300 shadow-sm p-12 text-center flex flex-col items-center justify-center gap-3"
    >
      <div class="w-14 h-14 rounded-full bg-base-200 flex items-center justify-center text-base-content/40">
        <Layers class="w-7 h-7" />
      </div>
      <h3 class="text-base font-bold">No Devices Found</h3>
      <p class="text-xs text-base-content/60 max-w-sm">
        {{
          searchQuery
            ? `No devices match "${searchQuery}". Clear your search query to see all devices.`
            : 'No devices found in this project.'
        }}
      </p>
      <button
        v-if="searchQuery"
        class="btn btn-sm btn-ghost mt-2"
        @click="searchQuery = ''"
      >
        Clear filter
      </button>
      <button
        v-else
        class="btn btn-sm btn-primary mt-2 gap-1.5"
        :disabled="store.isFetching"
        @click="store.fetchProject()"
      >
        <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': store.isFetching }" />
        Refresh Devices
      </button>
    </div>
  </div>
</template>
