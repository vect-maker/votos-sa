<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { RouterView, RouterLink } from 'vue-router'
import { useDevicesStore } from '@/stores/devices'
import { Activity, RefreshCw, Layers } from '@lucide/vue'

const store = useDevicesStore()

onMounted(() => {
  store.startLiveSync()
  store.fetchProject()
})

onUnmounted(() => {
  store.stopLiveSync()
})
</script>

<template>
  <div class="min-h-screen bg-base-200 text-base-content flex flex-col">
    <!-- Top Navigation Header -->
    <header class="bg-base-100 border-b border-base-300 shadow-sm sticky top-0 z-40">
      <div class="max-w-6xl mx-auto px-4 py-3 flex items-center justify-between gap-4">
        <!-- Brand / Home Link -->
        <RouterLink
          :to="{ name: 'devices-list' }"
          class="flex items-center gap-3 group"
        >
          <div class="w-9 h-9 rounded-xl bg-primary text-primary-content flex items-center justify-center shadow-sm group-hover:scale-105 transition-transform">
            <Activity class="w-5 h-5" />
          </div>
          <div>
            <div class="text-base font-bold tracking-tight flex items-center gap-2">
              <span>IoT Control Panel</span>
              <span class="badge badge-xs badge-primary font-normal">v1.0</span>
            </div>
            <p class="text-xs text-base-content/60 font-mono">
              {{ store.project?.name || 'Connecting...' }}
            </p>
          </div>
        </RouterLink>

        <!-- Right Header Status & Controls -->
        <div class="flex items-center gap-2.5">
          <!-- Live Status Badge -->
          <div
            class="badge gap-1.5 py-2.5 px-3 text-xs font-semibold"
            :class="store.isConnected ? 'badge-success text-success-content' : 'badge-warning text-warning-content'"
          >
            <span
              class="w-2 h-2 rounded-full"
              :class="store.isConnected ? 'bg-success-content animate-pulse' : 'bg-warning-content animate-ping'"
            ></span>
            <span class="hidden sm:inline">
              {{ store.isConnected ? 'Live' : 'Connecting...' }}
            </span>
            <span class="sm:hidden">
              {{ store.isConnected ? 'Live' : 'Connecting' }}
            </span>
          </div>

          <!-- Navigation link to Devices -->
          <RouterLink
            :to="{ name: 'devices-list' }"
            class="btn btn-ghost btn-sm gap-1.5 text-xs hidden md:inline-flex"
          >
            <Layers class="w-4 h-4" />
            <span>Devices</span>
          </RouterLink>

          <!-- Manual Refresh Button -->
          <button
            class="btn btn-outline btn-sm gap-1.5 text-xs"
            :disabled="store.isFetching"
            title="Refresh device data"
            @click="store.fetchProject()"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': store.isFetching }" />
            <span class="hidden sm:inline">Refresh</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Main Content Shell -->
    <main class="flex-1 max-w-6xl w-full mx-auto p-4 sm:p-6 lg:p-8">
      <!-- Global Error Notification -->
      <div v-if="store.error" class="alert alert-error shadow-lg mb-6">
        <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <div class="flex-1">
          <h3 class="font-bold text-sm">System Notification</h3>
          <div class="text-xs opacity-90">{{ store.error }}</div>
        </div>
        <button class="btn btn-xs btn-ghost" @click="store.error = null">Dismiss</button>
      </div>

      <!-- Router View with transition -->
      <RouterView v-slot="{ Component }">
        <transition name="fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </RouterView>
    </main>

    <!-- Simple Footer -->
    <footer class="py-4 border-t border-base-300 text-center text-xs text-base-content/50">
      <div class="max-w-6xl mx-auto px-4 flex flex-col sm:flex-row items-center justify-between gap-2">
        <span>IoT Device Management Panel</span>
        <span class="font-mono">Project ID: {{ store.project?.project_id || '...' }}</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
