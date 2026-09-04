import { ref } from 'vue'

export function useTts() {
  const isEnabled = ref(true)
  const isSupported = 'speechSynthesis' in window

  function speak(text: string, lang = 'en-US') {
    if (!isSupported || !isEnabled.value || !text.trim()) return

    window.speechSynthesis.cancel()

    const utterance = new SpeechSynthesisUtterance(text)
    utterance.lang = lang
    utterance.rate = 1.0
    utterance.pitch = 1.0

    window.speechSynthesis.speak(utterance)
  }

  function stop() {
    if (isSupported) window.speechSynthesis.cancel()
  }

  return { speak, stop, isEnabled, isSupported }
}
