<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { onMounted, onUnmounted } from 'vue'
import AppContent from './components/AppContent.vue'
import { useAppManager } from './composables/useAppManager'
import { useEventHandlers } from './composables/useEventHandlers'

// 使用封装的应用管理器
const {
  naiveTheme,
  mcpRequest,
  showMcpPopup,
  appConfig,
  isInitializing,
  actions,
} = useAppManager()

// 创建事件处理器
const handlers = useEventHandlers(actions)

// 浏览器监控弹窗监听器
let unlistenBrowserPopup: (() => void) | null = null

// 主题应用由useTheme统一管理，移除重复的主题应用逻辑

// 初始化
onMounted(async () => {
  try {
    await actions.app.initialize()

    // 监听浏览器 AI 完成弹窗事件
    unlistenBrowserPopup = await listen<{ site_name: string, url: string, title: string }>('show-browser-popup', async (event) => {
      const { site_name, url, title } = event.payload
      // 播放提示音
      try {
        await invoke('test_audio_sound')
      }
      catch (e) {
        console.log('播放提示音失败:', e)
      }
      // 显示系统通知
      if (Notification.permission === 'granted') {
        const notification = new Notification(`${site_name} AI 完成`, {
          body: title || '点击跳转到聊天页面',
          icon: '/icon.png',
        })
        notification.onclick = () => {
          invoke('open_browser_url', { url })
        }
      }
    })
  }
  catch (error) {
    console.error('应用初始化失败:', error)
  }
})

// 清理
onUnmounted(() => {
  actions.app.cleanup()
  if (unlistenBrowserPopup) {
    unlistenBrowserPopup()
  }
})
</script>

<template>
  <div class="min-h-screen bg-surface transition-colors duration-200">
    <n-config-provider :theme="naiveTheme">
      <n-message-provider>
        <n-notification-provider>
          <n-dialog-provider>
            <AppContent
              :mcp-request="mcpRequest" :show-mcp-popup="showMcpPopup" :app-config="appConfig"
              :is-initializing="isInitializing" @mcp-response="handlers.onMcpResponse" @mcp-cancel="handlers.onMcpCancel"
              @theme-change="handlers.onThemeChange" @toggle-always-on-top="handlers.onToggleAlwaysOnTop"
              @toggle-audio-notification="handlers.onToggleAudioNotification"
              @update-audio-url="handlers.onUpdateAudioUrl" @test-audio="handlers.onTestAudio"
              @stop-audio="handlers.onStopAudio" @test-audio-error="handlers.onTestAudioError"
              @update-window-size="handlers.onUpdateWindowSize"
              @update-reply-config="handlers.onUpdateReplyConfig" @message-ready="handlers.onMessageReady"
              @config-reloaded="handlers.onConfigReloaded"
            />
          </n-dialog-provider>
        </n-notification-provider>
      </n-message-provider>
    </n-config-provider>
  </div>
</template>
