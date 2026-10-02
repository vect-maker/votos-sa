export const TAG_BRIGHTNESS = 'brightness' as const
export const TAG_SERVO_X = 'servo_x' as const
export const TAG_SERVO_Y = 'servo_y' as const
export const TAG_LAMP = 'lamp' as const
export const TAG_FAN = 'fan' as const
export const TAG_LOCK = 'lock' as const
export const TAG_SERVO_HOME = 'servo_home' as const
export const TAG_CALIBRATE_LIGHT = 'calibrate_light' as const

export const ALL_TAGS = [
  TAG_BRIGHTNESS,
  TAG_SERVO_X,
  TAG_SERVO_Y,
  TAG_LAMP,
  TAG_FAN,
  TAG_LOCK,
  TAG_SERVO_HOME,
  TAG_CALIBRATE_LIGHT,
] as const

export const ACTUATOR_SWITCH_TAGS = [
  TAG_LAMP,
  TAG_FAN,
  TAG_LOCK,
] as const

export const ACTUATOR_BUTTON_TAGS = [
  TAG_SERVO_HOME,
  TAG_CALIBRATE_LIGHT,
] as const

export const ACTUATOR_SERVO_TAGS = [
  TAG_SERVO_X,
  TAG_SERVO_Y,
] as const

export const SERVO_MIN_ANGLE = 0
export const SERVO_MAX_ANGLE = 180

export const SERVO_X_MIN_ANGLE = 0
export const SERVO_X_MAX_ANGLE = 180

export const SERVO_Y_MIN_ANGLE = 0
export const SERVO_Y_MAX_ANGLE = 90

