import { createApp } from 'vue'
import App from './App.vue'
import SettingsApp from './SettingsApp.vue'
import ChatApp from './ChatApp.vue'
import './styles/global.css'

const params = new URLSearchParams(window.location.search)
const entry = params.get('window')

if (entry === 'settings') {
  createApp(SettingsApp).mount('#app')
} else if (entry === 'chat') {
  createApp(ChatApp).mount('#app')
} else {
  createApp(App).mount('#app')
}
