<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { useDevicesStore } from '@/stores/devices'
import LuxHistoryChart from '@/components/devices/LuxHistoryChart.vue'
import {
  TAG_BRIGHTNESS,
  TAG_LAMP,
  TAG_FAN,
  TAG_LOCK,
  TAG_SERVO_X,
  TAG_SERVO_Y,
  TAG_SERVO_HOME,
  TAG_CALIBRATE_LIGHT,
  SERVO_X_MIN_ANGLE,
  SERVO_X_MAX_ANGLE,
  SERVO_Y_MIN_ANGLE,
  SERVO_Y_MAX_ANGLE,
  ACTUATOR_SWITCH_TAGS,
  ACTUATOR_SERVO_TAGS,
  ACTUATOR_BUTTON_TAGS,
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
  RotateCcw,
  Radar,
  AlertCircle,
} from '@lucide/vue'

interface Props {
  id: string
}

const props = defineProps<Props>()
const store = useDevicesStore()

const commandLoading = ref<Record<string, boolean>>({})
const optimisticSwitches = ref<Record<string, boolean>>({})
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
  if (optimisticSwitches.value[tag] !== undefined) {
    return optimisticSwitches.value[tag]
  }
  const val = getSensorValue(tag)
  return val === true || val === 1 || val === '1' || val === 'true'
}

function getServoValue(tag: string): number {
  const val = getSensorValue(tag)
  const min = tag === TAG_SERVO_Y ? SERVO_Y_MIN_ANGLE : SERVO_X_MIN_ANGLE
  const max = tag === TAG_SERVO_Y ? SERVO_Y_MAX_ANGLE : SERVO_X_MAX_ANGLE
  const defaultVal = tag === TAG_SERVO_Y ? 45 : 90

  if (typeof val === 'number') {
    return Math.max(min, Math.min(max, val))
  }
  if (typeof val === 'string') {
    const parsed = parseInt(val, 10)
    if (!isNaN(parsed)) {
      return Math.max(min, Math.min(max, parsed))
    }
  }
  return defaultVal
}

async function toggleSwitch(tag: string) {
  if (!isOnline.value || !device.value) return
  const current = isSwitchActive(tag)
  const nextVal = !current

  // Optimistically set switch state immediately so UI responds instantaneously
  optimisticSwitches.value[tag] = nextVal
  commandLoading.value[tag] = true

  try {
    await store.sendControl({
      device_id: device.value.device_id,
      tag,
      value: nextVal ? 1 : 0,
    })
  } catch (err) {
    console.error(`Failed to toggle ${tag}:`, err)
    // Revert optimistic switch on failure
    optimisticSwitches.value[tag] = current
  } finally {
    commandLoading.value[tag] = false
    // Clear optimistic override once store has reconciled
    setTimeout(() => {
      delete optimisticSwitches.value[tag]
    }, 600)
  }
}

async function commitServo(tag: string, angle: number) {
  if (!isOnline.value || !device.value) return
  const min = tag === TAG_SERVO_Y ? SERVO_Y_MIN_ANGLE : SERVO_X_MIN_ANGLE
  const max = tag === TAG_SERVO_Y ? SERVO_Y_MAX_ANGLE : SERVO_X_MAX_ANGLE
  const clamped = Math.max(min, Math.min(max, angle))

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
    setTimeout(() => {
      if (tag === TAG_SERVO_X) localServoX.value = null
      if (tag === TAG_SERVO_Y) localServoY.value = null
    }, 200)
  }
}

const environmentalSensors = computed(() => {
  if (!device.value?.sensors) return {}
  const res: Record<string, string | number | boolean | null> = {}
  const excludedTags: readonly string[] = [
    TAG_BRIGHTNESS,
    ...ACTUATOR_SWITCH_TAGS,
    ...ACTUATOR_SERVO_TAGS,
    ...ACTUATOR_BUTTON_TAGS,
  ]
  for (const [tag, val] of Object.entries(device.value.sensors)) {
    if (!excludedTags.includes(tag)) {
      res[tag] = val
    }
  }
  return res
})

async function triggerHomePosition() {
  if (!isOnline.value || !device.value) return
  commandLoading.value[TAG_SERVO_HOME] = true
  localServoX.value = 90
  localServoY.value = 90

  try {
    await store.sendControl({
      device_id: device.value.device_id,
      tag: TAG_SERVO_HOME,
      value: 1,
    })
  } catch (err) {
    console.error(`Failed to send ${TAG_SERVO_HOME}:`, err)
  } finally {
    commandLoading.value[TAG_SERVO_HOME] = false
    setTimeout(() => {
      localServoX.value = null
      localServoY.value = null
    }, 600)
  }
}

async function triggerCalibrateLight() {
  if (!isOnline.value || !device.value) return
  commandLoading.value[TAG_CALIBRATE_LIGHT] = true

  try {
    await store.sendControl({
      device_id: device.value.device_id,
      tag: TAG_CALIBRATE_LIGHT,
      value: 1,
    })
  } catch (err) {
    console.error(`Failed to send ${TAG_CALIBRATE_LIGHT}:`, err)
  } finally {
    commandLoading.value[TAG_CALIBRATE_LIGHT] = false
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
            <!-- Ambient Light Sensor Gauge -->
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

            <!-- Additional Environmental Sensors (if present) -->
            <template v-for="(val, tag) in environmentalSensors" :key="tag">
              <div
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

        <!-- Real-time Ambient Light (Lux) History Graph -->
        <LuxHistoryChart v-if="device" :device-id="device.device_id" />

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

                <div class="flex items-center gap-2.5">
                  <span
                    v-if="commandLoading[TAG_LAMP]"
                    class="loading loading-spinner loading-xs text-primary"
                    title="Updating lamp..."
                  ></span>
                  <TooltipRoot :delay-duration="200">
                    <TooltipTrigger as-child>
                      <div>
                        <SwitchRoot
                          :model-value="isSwitchActive(TAG_LAMP)"
                          :disabled="!isOnline || commandLoading[TAG_LAMP]"
                          aria-label="Toggle Lamp / Main Light"
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

                <div class="flex items-center gap-2.5">
                  <span
                    v-if="commandLoading[TAG_FAN]"
                    class="loading loading-spinner loading-xs text-primary"
                    title="Updating fan..."
                  ></span>
                  <TooltipRoot :delay-duration="200">
                    <TooltipTrigger as-child>
                      <div>
                        <SwitchRoot
                          :model-value="isSwitchActive(TAG_FAN)"
                          :disabled="!isOnline || commandLoading[TAG_FAN]"
                          aria-label="Toggle Ventilation Fan"
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

                <div class="flex items-center gap-2.5">
                  <span
                    v-if="commandLoading[TAG_LOCK]"
                    class="loading loading-spinner loading-xs text-primary"
                    title="Updating lock..."
                  ></span>
                  <TooltipRoot :delay-duration="200">
                    <TooltipTrigger as-child>
                      <div>
                        <SwitchRoot
                          :model-value="isSwitchActive(TAG_LOCK)"
                          :disabled="!isOnline || commandLoading[TAG_LOCK]"
                          aria-label="Toggle Smart Lock"
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
                  <div class="flex items-center gap-2">
                    <span
                      v-if="commandLoading[TAG_SERVO_X]"
                      class="loading loading-spinner loading-xs text-primary"
                      title="Rotating servo X..."
                    ></span>
                    <span class="badge badge-sm badge-neutral font-mono">
                      {{ localServoX ?? getServoValue(TAG_SERVO_X) }}°
                    </span>
                  </div>
                </div>

                <!-- Slider -->
                <SliderRoot
                  :model-value="[localServoX ?? getServoValue(TAG_SERVO_X)]"
                  :min="SERVO_X_MIN_ANGLE"
                  :max="SERVO_X_MAX_ANGLE"
                  :step="1"
                  :disabled="!isOnline || commandLoading[TAG_SERVO_X]"
                  aria-label="Servo Horizon (X) angle"
                  class="relative flex items-center select-none touch-none w-full h-5 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
                  @update:model-value="onServoXUpdate"
                  @value-commit="onServoXCommit"
                >
                  <SliderTrack class="bg-base-300 relative grow rounded-full h-2">
                    <SliderRange class="absolute bg-primary rounded-full h-full" />
                  </SliderTrack>
                  <SliderThumb
                    class="block w-5 h-5 bg-primary rounded-full shadow focus:outline-none focus:ring-2 focus:ring-primary/50 cursor-grab active:cursor-grabbing relative before:absolute before:-inset-2.5 before:content-['']"
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
                  <div class="flex items-center gap-2">
                    <span
                      v-if="commandLoading[TAG_SERVO_Y]"
                      class="loading loading-spinner loading-xs text-primary"
                      title="Rotating servo Y..."
                    ></span>
                    <span class="badge badge-sm badge-neutral font-mono">
                      {{ localServoY ?? getServoValue(TAG_SERVO_Y) }}°
                    </span>
                  </div>
                </div>

                <!-- Slider -->
                <SliderRoot
                  :model-value="[localServoY ?? getServoValue(TAG_SERVO_Y)]"
                  :min="SERVO_Y_MIN_ANGLE"
                  :max="SERVO_Y_MAX_ANGLE"
                  :step="1"
                  :disabled="!isOnline || commandLoading[TAG_SERVO_Y]"
                  aria-label="Servo Vertical (Y) angle"
                  class="relative flex items-center select-none touch-none w-full h-5 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
                  @update:model-value="onServoYUpdate"
                  @value-commit="onServoYCommit"
                >
                  <SliderTrack class="bg-base-300 relative grow rounded-full h-2">
                    <SliderRange class="absolute bg-primary rounded-full h-full" />
                  </SliderTrack>
                  <SliderThumb
                    class="block w-5 h-5 bg-primary rounded-full shadow focus:outline-none focus:ring-2 focus:ring-primary/50 cursor-grab active:cursor-grabbing relative before:absolute before:-inset-2.5 before:content-['']"
                  />
                </SliderRoot>

                <div class="flex justify-between text-[10px] text-base-content/50 px-1 font-mono">
                  <span>0°</span>
                  <span>45°</span>
                  <span>90°</span>
                </div>
              </div>

              <!-- Home Servos (Momentary Action Button) -->
              <div class="p-3.5 bg-base-200/60 rounded-box border border-base-300/60 flex items-center justify-between gap-3">
                <div class="flex items-center gap-3">
                  <div class="w-9 h-9 rounded-lg bg-base-300 flex items-center justify-center text-primary shrink-0">
                    <RotateCcw class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="text-sm font-semibold">Home Position</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_SERVO_HOME }} • 90°, 90°</div>
                  </div>
                </div>

                <button
                  class="btn btn-sm btn-outline btn-primary gap-1.5"
                  :disabled="!isOnline || commandLoading[TAG_SERVO_HOME]"
                  @click="triggerHomePosition"
                >
                  <span
                    v-if="commandLoading[TAG_SERVO_HOME]"
                    class="loading loading-spinner loading-xs"
                  ></span>
                  <RotateCcw v-else class="w-3.5 h-3.5" />
                  <span>Reset Servos</span>
                </button>
              </div>

              <!-- Light Calibration / Mapping (Momentary Action Button) -->
              <div class="p-3.5 bg-base-200/60 rounded-box border border-base-300/60 flex items-center justify-between gap-3">
                <div class="flex items-center gap-3">
                  <div class="w-9 h-9 rounded-lg bg-warning/10 text-warning flex items-center justify-center shrink-0">
                    <Radar class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="text-sm font-semibold">Calibrate Light Tracking</div>
                    <div class="text-[10px] text-base-content/50 font-mono">tag: {{ TAG_CALIBRATE_LIGHT }} • Auto-find max lux</div>
                  </div>
                </div>

                <button
                  class="btn btn-sm btn-outline btn-warning gap-1.5"
                  :disabled="!isOnline || commandLoading[TAG_CALIBRATE_LIGHT]"
                  @click="triggerCalibrateLight"
                >
                  <span
                    v-if="commandLoading[TAG_CALIBRATE_LIGHT]"
                    class="loading loading-spinner loading-xs"
                  ></span>
                  <Radar v-else class="w-3.5 h-3.5" />
                  <span>Map Light</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </template>
    </TooltipProvider>
  </div>
</template>
