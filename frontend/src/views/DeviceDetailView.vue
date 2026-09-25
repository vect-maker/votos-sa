<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { useDevicesStore } from '@/stores/devices'
import {
  TAG_BRIGHTNESS,
  TAG_LAMP,
  TAG_FAN,
  TAG_LOCK,
  TAG_SERVO_X,
  TAG_SERVO_Y,
  SERVO_MIN_ANGLE,
  SERVO_MAX_ANGLE,
} from '@/api/modules/devices/domain'
import {
  SwitchRoot,
  SwitchThumb,
  SliderRoot,
  SliderTrack,
  SliderRange,
  SliderThumb,
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
  TooltipContent,
  TooltipPortal,
  TooltipArrow,
} from 'reka-ui'
import {
  ArrowLeft,
  Cpu,
  Wifi,
  WifiOff,
  Sun,
  Lightbulb,
  Fan,
  Lock,
  Unlock,
  Activity,
  Sliders,
  AlertCircle,
} from '@lucide/vue'

interface Props {
  id: string
}

const props = defineProps<Props>()
const store = useDevicesStore()

const commandLoading = ref<Record<string, boolean>>({})
const localServoX = ref<number | null>(null)
const localServoY = ref<number | null>(null)

const numericId = computed(() => parseInt(props.id, 10))

const device = computed(() => {
  return store.getDeviceById(numericId.value)
})

const isOnline = computed(() => {
  return device.value?.sensors != null
})

onMounted(() => {
  if (!store.project && !store.isFetching) {
    store.fetchProject()
  }
})

function getSensorValue(tag: string): string | number | boolean | null {
  if (!device.value?.sensors) return null
  return device.value.sensors[tag] ?? null
}

function isSwitchActive(tag: string): boolean {
  const val = getSensorValue(tag)
  return val === true || val === 1 || val === '1' || val === 'true'
}

function getServoValue(tag: string): number {
  const val = getSensorValue(tag)
  if (typeof val === 'number') {
    return Math.max(SERVO_MIN_ANGLE, Math.min(SERVO_MAX_ANGLE, val))
  }
  if (typeof val === 'string') {
    const parsed = parseInt(val, 10)
    if (!isNaN(parsed)) {
      return Math.max(SERVO_MIN_ANGLE, Math.min(SERVO_MAX_ANGLE, parsed))
    }
  }
  return 90
}

async function toggleSwitch(tag: string) {
  if (!isOnline.value || !device.value) return
  const current = isSwitchActive(tag)
  const nextVal = !current ? 1 : 0

  commandLoading.value[tag] = true
  try {
    await store.sendControl({
      device_id: device.value.device_id,
      tag,
      value: nextVal,
    })
  } catch (err) {
    console.error(`Failed to toggle ${tag}:`, err)
  } finally {
    commandLoading.value[tag] = false
  }
}

async function commitServo(tag: string, angle: number) {
  if (!isOnline.value || !device.value) return
  const clamped = Math.max(SERVO_MIN_ANGLE, Math.min(SERVO_MAX_ANGLE, angle))

  commandLoading.value[tag] = true
  try {
    await store.sendControl({
      device_id: device.value.device_id,
      tag,
      value: clamped,
    })
  } catch (err) {
    console.error(`Failed to rotate ${tag}:`, err)
  } finally {
    commandLoading.value[tag] = false
    if (tag === TAG_SERVO_X) localServoX.value = null
    if (tag === TAG_SERVO_Y) localServoY.value = null
  }
}

function onServoXUpdate(values?: number[]) {
  if (values && typeof values[0] === 'number') {
    localServoX.value = values[0]
  }
}

function onServoXCommit(values?: number[]) {
  if (values && typeof values[0] === 'number') {
    commitServo(TAG_SERVO_X, values[0])
  }
}

function onServoYUpdate(values?: number[]) {
  if (values && typeof values[0] === 'number') {
    localServoY.value = values[0]
  }
}

function onServoYCommit(values?: number[]) {
  if (values && typeof values[0] === 'number') {
    commitServo(TAG_SERVO_Y, values[0])
  }
}

const brightnessValue = computed<number | null>(() => {
  const raw = getSensorValue(TAG_BRIGHTNESS)
  if (typeof raw === 'number') return Math.round(raw)
  if (typeof raw === 'string') {
    const parsed = parseFloat(raw)
    return isNaN(parsed) ? null : Math.round(parsed)
  }
  return null
})
</script>

<template>
  <div class="flex flex-col gap-6">
    <TooltipProvider>
      <!-- Top Navigation & Breadcrumbs -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 bg-base-100 p-4 rounded-box border border-base-300 shadow-sm">
        <div class="flex items-center gap-3">
          <RouterLink
            :to="{ name: 'devices-list' }"
            class="btn btn-sm btn-ghost gap-1.5 text-base-content/80 hover:text-base-content"
          >
            <ArrowLeft class="w-4 h-4" />
            <span>Devices List</span>
          </RouterLink>

          <div class="divider divider-horizontal my-0"></div>

          <div class="text-xs breadcrumbs p-0">
            <ul>
              <li>
                <RouterLink :to="{ name: 'devices-list' }">Devices</RouterLink>
              </li>
              <li class="font-semibold text-base-content">
                {{ device?.name || `Device #${props.id}` }}
              </li>
            </ul>
          </div>
        </div>

        <div v-if="device" class="flex items-center gap-2">
          <!-- Status Badge -->
          <div
            class="badge gap-1.5 py-2.5 px-3 text-xs font-semibold"
            :class="isOnline ? 'badge-success text-success-content' : 'badge-ghost text-base-content/60'"
          >
            <span
              v-if="isOnline"
              class="w-2 h-2 rounded-full bg-success-content animate-pulse"
            ></span>
            <component :is="isOnline ? Wifi : WifiOff" class="w-3.5 h-3.5" />
            {{ isOnline ? 'Device Online' : 'Device Offline' }}
          </div>
        </div>
      </div>

      <!-- Device Not Found / Still Loading -->
      <div
        v-if="!device"
        class="card bg-base-100 border border-base-300 shadow-sm p-12 text-center flex flex-col items-center justify-center gap-3"
      >
        <div class="w-14 h-14 rounded-full bg-base-200 flex items-center justify-center text-base-content/40">
          <Cpu class="w-7 h-7" />
        </div>
        <h3 class="text-base font-bold">
          {{ store.isFetching ? 'Loading Device Telemetry...' : `Device #${props.id} Not Found` }}
        </h3>
        <p class="text-xs text-base-content/60 max-w-sm">
          {{
            store.isFetching
              ? 'Loading device details...'
              : 'Could not locate this device in the active project. Return to the devices list.'
          }}
        </p>
        <RouterLink
          :to="{ name: 'devices-list' }"
          class="btn btn-sm btn-primary mt-2 gap-1.5"
        >
          <ArrowLeft class="w-4 h-4" />
          Back to Devices List
        </RouterLink>
      </div>

      <!-- Device Content -->
      <template v-else>
        <!-- Device Info Banner -->
        <div class="card bg-base-100 border border-base-300 shadow-sm p-5">
          <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
            <div class="flex items-center gap-3.5">
              <div
                class="w-12 h-12 rounded-2xl flex items-center justify-center transition-colors"
                :class="isOnline ? 'bg-primary/10 text-primary' : 'bg-base-200 text-base-content/50'"
              >
                <Cpu class="w-6 h-6" />
              </div>
              <div>
                <h1 class="text-xl font-bold tracking-tight">{{ device.name }}</h1>
                <div class="flex items-center gap-2 text-xs text-base-content/60 font-mono mt-0.5">
                  <span>Device ID: #{{ device.device_id }}</span>
                  <span>•</span>
                  <span class="badge badge-sm badge-ghost font-mono">Tag: {{ device.tag }}</span>
                </div>
              </div>
            </div>

            <!-- Offline Notice Alert -->
            <div
              v-if="!isOnline"
              class="alert alert-warning py-2.5 px-4 text-xs shadow-sm max-w-md"
            >
              <AlertCircle class="w-4 h-4 shrink-0" />
              <span>
                <strong>Device Offline:</strong> The device is currently disconnected or powered off. Controls are disabled.
              </span>
            </div>
          </div>
        </div>

        <!-- Telemetry & Sensor Readings -->
        <div class="card bg-base-100 border border-base-300 shadow-sm p-5">
          <div class="flex items-center justify-between mb-4 border-b border-base-200 pb-3">
            <div class="flex items-center gap-2">
              <Activity class="w-5 h-5 text-primary" />
              <h2 class="text-base font-bold">Telemetry & Sensors</h2>
            </div>
            <span class="text-xs text-base-content/60 font-medium">
              {{ isOnline ? 'Live Realtime Stream' : 'Sensors Inactive' }}
            </span>
          </div>

          <div v-if="isOnline && device.sensors" class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
            <!-- Brightness Gauge -->
            <div class="bg-base-200/60 border border-base-300/60 rounded-box p-4 flex items-center justify-between">
              <div>
                <div class="flex items-center gap-1.5 text-xs text-base-content/60 font-medium">
                  <Sun class="w-4 h-4 text-warning" />
                  <span>Ambient Light</span>
                </div>
                <div class="text-2xl font-bold mt-1">
                  {{ brightnessValue !== null ? `${brightnessValue} lx` : 'N/A' }}
                </div>
                <div class="text-[10px] text-base-content/50 font-mono mt-0.5">tag: {{ TAG_BRIGHTNESS }}</div>
              </div>

              <div
                v-if="brightnessValue !== null"
                class="radial-progress text-warning text-xs font-bold"
                :style="`--value:${Math.min(100, Math.round((brightnessValue / 1000) * 100))}; --size:3.5rem; --thickness:4px;`"
                role="progressbar"
              >
                {{ Math.min(100, Math.round((brightnessValue / 1000) * 100)) }}%
              </div>
            </div>

            <!-- Other Telemetry Keys -->
            <template v-for="(val, tag) in device.sensors" :key="tag">
              <div
                v-if="tag !== TAG_BRIGHTNESS"
                class="bg-base-200/60 border border-base-300/60 rounded-box p-4 flex flex-col justify-between"
              >
                <div class="flex items-center justify-between text-xs text-base-content/60 font-medium">
                  <span>{{ tag }}</span>
                  <Activity class="w-3.5 h-3.5 text-primary" />
                </div>
                <div class="text-xl font-bold mt-2 font-mono">
                  {{ val !== null ? String(val) : 'null' }}
                </div>
                <div class="text-[10px] text-base-content/50 font-mono mt-1">live value</div>
              </div>
            </template>
          </div>

          <!-- Empty Telemetry when offline -->
          <div
            v-else
            class="p-6 text-center bg-base-200/40 rounded-box border border-dashed border-base-300 flex flex-col items-center justify-center gap-2"
          >
            <WifiOff class="w-6 h-6 text-base-content/30" />
            <p class="text-xs text-base-content/60 font-medium">
              No real-time sensor telemetry is being received because this device is offline.
            </p>
          </div>
        </div>

        <!-- Controls & Actuators -->
        <div class="card bg-base-100 border border-base-300 shadow-sm p-5">
          <div class="flex items-center justify-between mb-4 border-b border-base-200 pb-3">
            <div class="flex items-center gap-2">
              <Sliders class="w-5 h-5 text-primary" />
              <h2 class="text-base font-bold">Actuators & Controls</h2>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
            <!-- Binary Switches -->
            <div class="flex flex-col gap-3">
              <h3 class="text-xs font-semibold uppercase tracking-wider text-base-content/60">
                Binary Switches
              </h3>

              <!-- Lamp Switch -->
              <div class="flex items-center justify-between p-3.5 bg-base-200/60 rounded-box border border-base-300/60">
                <div class="flex items-center gap-3">
                  <div
                    class="w-9 h-9 rounded-lg flex items-center justify-center transition-colors"
                    :class="isSwitchActive(TAG_LAMP) ? 'bg-warning text-warning-content' : 'bg-base-300 text-base-content/60'"
                  >
                    <Lightbulb class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="text-sm font-semibold">Lamp / Main Light</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_LAMP }}</div>
                  </div>
                </div>

                <TooltipRoot :delay-duration="200">
                  <TooltipTrigger as-child>
                    <div>
                      <SwitchRoot
                        :model-value="isSwitchActive(TAG_LAMP)"
                        :disabled="!isOnline || commandLoading[TAG_LAMP]"
                        class="w-12 h-6 bg-base-300 rounded-full relative transition-colors cursor-pointer data-[state=checked]:bg-primary disabled:cursor-not-allowed disabled:opacity-50 inline-flex items-center px-0.5"
                        @update:model-value="toggleSwitch(TAG_LAMP)"
                      >
                        <SwitchThumb
                          class="block w-5 h-5 bg-white rounded-full shadow-md transition-transform duration-150 translate-x-0 data-[state=checked]:translate-x-6"
                        />
                      </SwitchRoot>
                    </div>
                  </TooltipTrigger>
                  <TooltipPortal v-if="!isOnline">
                    <TooltipContent class="bg-neutral text-neutral-content text-xs px-2 py-1 rounded shadow-md z-50">
                      Device is offline. Controls disabled.
                      <TooltipArrow class="fill-neutral" />
                    </TooltipContent>
                  </TooltipPortal>
                </TooltipRoot>
              </div>

              <!-- Ventilation Fan Switch -->
              <div class="flex items-center justify-between p-3.5 bg-base-200/60 rounded-box border border-base-300/60">
                <div class="flex items-center gap-3">
                  <div
                    class="w-9 h-9 rounded-lg flex items-center justify-center transition-colors"
                    :class="isSwitchActive(TAG_FAN) ? 'bg-primary text-primary-content animate-spin' : 'bg-base-300 text-base-content/60'"
                  >
                    <Fan class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="text-sm font-semibold">Ventilation Fan</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_FAN }}</div>
                  </div>
                </div>

                <TooltipRoot :delay-duration="200">
                  <TooltipTrigger as-child>
                    <div>
                      <SwitchRoot
                        :model-value="isSwitchActive(TAG_FAN)"
                        :disabled="!isOnline || commandLoading[TAG_FAN]"
                        class="w-12 h-6 bg-base-300 rounded-full relative transition-colors cursor-pointer data-[state=checked]:bg-primary disabled:cursor-not-allowed disabled:opacity-50 inline-flex items-center px-0.5"
                        @update:model-value="toggleSwitch(TAG_FAN)"
                      >
                        <SwitchThumb
                          class="block w-5 h-5 bg-white rounded-full shadow-md transition-transform duration-150 translate-x-0 data-[state=checked]:translate-x-6"
                        />
                      </SwitchRoot>
                    </div>
                  </TooltipTrigger>
                  <TooltipPortal v-if="!isOnline">
                    <TooltipContent class="bg-neutral text-neutral-content text-xs px-2 py-1 rounded shadow-md z-50">
                      Device is offline. Controls disabled.
                      <TooltipArrow class="fill-neutral" />
                    </TooltipContent>
                  </TooltipPortal>
                </TooltipRoot>
              </div>

              <!-- Lock Switch -->
              <div class="flex items-center justify-between p-3.5 bg-base-200/60 rounded-box border border-base-300/60">
                <div class="flex items-center gap-3">
                  <div
                    class="w-9 h-9 rounded-lg flex items-center justify-center transition-colors"
                    :class="isSwitchActive(TAG_LOCK) ? 'bg-success text-success-content' : 'bg-base-300 text-base-content/60'"
                  >
                    <component :is="isSwitchActive(TAG_LOCK) ? Lock : Unlock" class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="text-sm font-semibold">Smart Lock</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_LOCK }}</div>
                  </div>
                </div>

                <TooltipRoot :delay-duration="200">
                  <TooltipTrigger as-child>
                    <div>
                      <SwitchRoot
                        :model-value="isSwitchActive(TAG_LOCK)"
                        :disabled="!isOnline || commandLoading[TAG_LOCK]"
                        class="w-12 h-6 bg-base-300 rounded-full relative transition-colors cursor-pointer data-[state=checked]:bg-primary disabled:cursor-not-allowed disabled:opacity-50 inline-flex items-center px-0.5"
                        @update:model-value="toggleSwitch(TAG_LOCK)"
                      >
                        <SwitchThumb
                          class="block w-5 h-5 bg-white rounded-full shadow-md transition-transform duration-150 translate-x-0 data-[state=checked]:translate-x-6"
                        />
                      </SwitchRoot>
                    </div>
                  </TooltipTrigger>
                  <TooltipPortal v-if="!isOnline">
                    <TooltipContent class="bg-neutral text-neutral-content text-xs px-2 py-1 rounded shadow-md z-50">
                      Device is offline. Controls disabled.
                      <TooltipArrow class="fill-neutral" />
                    </TooltipContent>
                  </TooltipPortal>
                </TooltipRoot>
              </div>
            </div>

            <!-- Servo Sliders -->
            <div class="flex flex-col gap-3">
              <h3 class="text-xs font-semibold uppercase tracking-wider text-base-content/60">
                Servo Actuators
              </h3>

              <!-- Servo X -->
              <div class="p-3.5 bg-base-200/60 rounded-box border border-base-300/60 flex flex-col gap-3">
                <div class="flex items-center justify-between">
                  <div>
                    <div class="text-sm font-semibold">Servo Horizon (X)</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_SERVO_X }}</div>
                  </div>
                  <span class="badge badge-sm badge-neutral font-mono">
                    {{ localServoX ?? getServoValue(TAG_SERVO_X) }}°
                  </span>
                </div>

                <!-- Slider -->
                <SliderRoot
                  :model-value="[localServoX ?? getServoValue(TAG_SERVO_X)]"
                  :min="SERVO_MIN_ANGLE"
                  :max="SERVO_MAX_ANGLE"
                  :step="1"
                  :disabled="!isOnline || commandLoading[TAG_SERVO_X]"
                  class="relative flex items-center select-none touch-none w-full h-5 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
                  @update:model-value="onServoXUpdate"
                  @value-commit="onServoXCommit"
                >
                  <SliderTrack class="bg-base-300 relative grow rounded-full h-2">
                    <SliderRange class="absolute bg-primary rounded-full h-full" />
                  </SliderTrack>
                  <SliderThumb
                    class="block w-5 h-5 bg-primary rounded-full shadow focus:outline-none focus:ring-2 focus:ring-primary/50"
                  />
                </SliderRoot>

                <div class="flex justify-between text-[10px] text-base-content/50 px-1 font-mono">
                  <span>0°</span>
                  <span>90°</span>
                  <span>180°</span>
                </div>
              </div>

              <!-- Servo Y -->
              <div class="p-3.5 bg-base-200/60 rounded-box border border-base-300/60 flex flex-col gap-3">
                <div class="flex items-center justify-between">
                  <div>
                    <div class="text-sm font-semibold">Servo Vertical (Y)</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_SERVO_Y }}</div>
                  </div>
                  <span class="badge badge-sm badge-neutral font-mono">
                    {{ localServoY ?? getServoValue(TAG_SERVO_Y) }}°
                  </span>
                </div>

                <!-- Slider -->
                <SliderRoot
                  :model-value="[localServoY ?? getServoValue(TAG_SERVO_Y)]"
                  :min="SERVO_MIN_ANGLE"
                  :max="SERVO_MAX_ANGLE"
                  :step="1"
                  :disabled="!isOnline || commandLoading[TAG_SERVO_Y]"
                  class="relative flex items-center select-none touch-none w-full h-5 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
                  @update:model-value="onServoYUpdate"
                  @value-commit="onServoYCommit"
                >
                  <SliderTrack class="bg-base-300 relative grow rounded-full h-2">
                    <SliderRange class="absolute bg-primary rounded-full h-full" />
                  </SliderTrack>
                  <SliderThumb
                    class="block w-5 h-5 bg-primary rounded-full shadow focus:outline-none focus:ring-2 focus:ring-primary/50"
                  />
                </SliderRoot>

                <div class="flex justify-between text-[10px] text-base-content/50 px-1 font-mono">
                  <span>0°</span>
                  <span>90°</span>
                  <span>180°</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>
    </TooltipProvider>
  </div>
</template>
