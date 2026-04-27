import { createApp } from 'vue'
import App from './App.vue'
import SettingsApp from './SettingsApp.vue'
import ChatApp from './ChatApp.vue'
import StatusApp from './StatusApp.vue'
import OnboardingApp from './OnboardingApp.vue'
import './styles/global.css'
import './composables/useKeyboardFix'

const params = new URLSearchParams(window.location.search)
const entry = params.get('window')

if (entry === 'settings') {
  createApp(SettingsApp).mount('#app')
} else if (entry === 'chat') {
  createApp(ChatApp).mount('#app')
} else if (entry === 'status') {
  createApp(StatusApp).mount('#app')
} else if (entry === 'onboarding') {
  createApp(OnboardingApp).mount('#app')
} else {
  createApp(App).mount('#app')
}
