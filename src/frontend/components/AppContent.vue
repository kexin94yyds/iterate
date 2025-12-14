<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useMessage } from 'naive-ui'
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { setupExitWarningListener } from '../composables/useExitWarning'
import { useKeyboard } from '../composables/useKeyboard'
import { useVersionCheck } from '../composables/useVersionCheck'
import UpdateModal from './common/UpdateModal.vue'
import LayoutWrapper from './layout/LayoutWrapper.vue'
import McpPopup from './popup/McpPopup.vue'
import PopupHeader from './popup/PopupHeader.vue'

interface AppConfig {
  theme: string
  window: {
    alwaysOnTop: boolean
    width: number
    height: number
    fixed: boolean
  }
  audio: {
    enabled: boolean
    url: string
  }
  reply: {
    enabled: boolean
    prompt: string
  }
}

interface Props {
  mcpRequest: any
  showMcpPopup: boolean
  appConfig: AppConfig
  isInitializing: boolean
}

interface Emits {
  mcpResponse: [response: any]
  mcpCancel: []
  themeChange: [theme: string]
  toggleAlwaysOnTop: []
  toggleAudioNotification: []
  updateAudioUrl: [url: string]
  testAudio: []
  stopAudio: []
  testAudioError: [error: any]
  updateWindowSize: [size: { width: number, height: number, fixed: boolean }]
  updateReplyConfig: [config: { enable_continue_reply?: boolean, continue_prompt?: string }]
  messageReady: [message: any]
  configReloaded: []
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

// 版本检查相关
const { versionInfo, showUpdateModal } = useVersionCheck()

// 弹窗中的设置显示控制
const showPopupSettings = ref(false)

// 窗口最小化功能
async function minimizeWindow() {
  try {
    const window = getCurrentWindow()
    await window.minimize()
  }
  catch (error) {
    console.error('最小化窗口失败:', error)
  }
}

// 初始化 Naive UI 消息实例
const message = useMessage()

// 键盘快捷键处理
const { handleExitShortcut } = useKeyboard()

// 切换弹窗设置显示
function togglePopupSettings() {
  showPopupSettings.value = !showPopupSettings.value
}

// 监听 MCP 请求变化，当有新请求时重置设置页面状态并更新窗口注册
watch(() => props.mcpRequest, async (newRequest) => {
  if (newRequest && showPopupSettings.value) {
    // 有新的 MCP 请求时，自动切换回消息页面
    showPopupSettings.value = false
  }

  // 更新窗口注册（使用新的项目路径）
  if (newRequest?.project_path) {
    try {
      await invoke('register_window_instance', { projectPath: newRequest.project_path })
    }
    catch (error) {
      console.error('更新窗口注册失败:', error)
    }
  }
}, { immediate: true })

// 全局键盘事件处理器
function handleGlobalKeydown(event: KeyboardEvent) {
  // Shift+Tab 切换置顶 - 仅在 MCP 弹窗显示时生效
  if (event.key === 'Tab' && event.shiftKey && props.showMcpPopup) {
    event.preventDefault()
    emit('toggleAlwaysOnTop')
    return
  }

  // Tab 键最小化当前弹窗 - 仅在 MCP 弹窗显示时生效
  if (event.key === 'Tab' && props.showMcpPopup) {
    event.preventDefault()
    minimizeWindow()
    return
  }

  handleExitShortcut(event)
}

onMounted(async () => {
  // 将消息实例传递给父组件
  emit('messageReady', message)
  // 设置退出警告监听器（统一处理主界面和弹窗）
  setupExitWarningListener(message)

  // 添加全局键盘事件监听器
  document.addEventListener('keydown', handleGlobalKeydown)

  // 注册当前窗口实例
  try {
    const projectPath = props.mcpRequest?.project_path || 'Unknown'
    await invoke('register_window_instance', { projectPath })
  }
  catch (error) {
    console.error('注册窗口实例失败:', error)
  }
})

onUnmounted(async () => {
  // 移除键盘事件监听器
  document.removeEventListener('keydown', handleGlobalKeydown)

  // 注销当前窗口实例
  try {
    await invoke('unregister_window_instance')
  }
  catch (error) {
    console.error('注销窗口实例失败:', error)
  }
})
</script>

<template>
  <div class="min-h-screen bg-black">
    <!-- MCP弹窗模式 -->
    <div
      v-if="props.showMcpPopup && props.mcpRequest"
      class="flex flex-col w-full h-screen bg-black text-white select-none"
    >
      <!-- 头部 - 固定在顶部 -->
      <div class="sticky top-0 z-50 flex-shrink-0 bg-black-100 border-b-2 border-black-200">
        <PopupHeader
          :current-theme="props.appConfig.theme"
          :loading="false"
          :show-main-layout="showPopupSettings"
          :always-on-top="props.appConfig.window.alwaysOnTop"
          :project-path="props.mcpRequest?.project_path"
          @theme-change="$emit('themeChange', $event)"
          @open-main-layout="togglePopupSettings"
          @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
        />
      </div>

      <!-- 设置界面 -->
      <div
        v-if="showPopupSettings"
        class="flex-1 overflow-y-auto scrollbar-thin"
      >
        <LayoutWrapper
          :app-config="props.appConfig"
          @theme-change="$emit('themeChange', $event)"
          @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
          @toggle-audio-notification="$emit('toggleAudioNotification')"
          @update-audio-url="$emit('updateAudioUrl', $event)"
          @test-audio="$emit('testAudio')"
          @stop-audio="$emit('stopAudio')"
          @test-audio-error="$emit('testAudioError', $event)"
          @update-window-size="$emit('updateWindowSize', $event)"
        />
      </div>

      <!-- 弹窗内容 -->
      <McpPopup
        v-else
        :request="props.mcpRequest"
        :app-config="props.appConfig"
        @response="$emit('mcpResponse', $event)"
        @cancel="$emit('mcpCancel')"
        @theme-change="$emit('themeChange', $event)"
      />
    </div>

    <!-- 弹窗加载骨架屏 或 初始化骨架屏 -->
    <div
      v-else-if="props.showMcpPopup || props.isInitializing"
      class="flex flex-col w-full h-screen bg-black text-white"
    >
      <!-- 头部骨架 -->
      <div class="flex-shrink-0 bg-black-100 border-b-2 border-black-200 px-4 py-3">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <n-skeleton
              circle
              :width="12"
              :height="12"
            />
            <n-skeleton
              text
              :width="256"
            />
          </div>
          <div class="flex gap-2">
            <n-skeleton
              circle
              :width="32"
              :height="32"
            />
            <n-skeleton
              circle
              :width="32"
              :height="32"
            />
          </div>
        </div>
      </div>

      <!-- 内容骨架 -->
      <div class="flex-1 p-4">
        <div class="bg-black-100 rounded-lg p-4 mb-4">
          <n-skeleton
            text
            :repeat="3"
          />
        </div>

        <div class="space-y-3">
          <n-skeleton
            text
            :width="128"
          />
          <n-skeleton
            text
            :repeat="3"
          />
        </div>
      </div>

      <!-- 底部骨架 -->
      <div class="flex-shrink-0 bg-black-100 border-t-2 border-black-200 p-4">
        <div class="flex justify-between items-center">
          <n-skeleton
            text
            :width="96"
          />
          <div class="flex gap-2">
            <n-skeleton
              text
              :width="64"
              :height="32"
            />
            <n-skeleton
              text
              :width="64"
              :height="32"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- 主界面 - 只在非弹窗模式且非初始化时显示 -->
    <LayoutWrapper
      v-else
      :app-config="props.appConfig"
      @theme-change="$emit('themeChange', $event)"
      @toggle-always-on-top="$emit('toggleAlwaysOnTop')"
      @toggle-audio-notification="$emit('toggleAudioNotification')"
      @update-audio-url="$emit('updateAudioUrl', $event)"
      @test-audio="$emit('testAudio')"
      @stop-audio="$emit('stopAudio')"
      @test-audio-error="$emit('testAudioError', $event)"
      @update-window-size="$emit('updateWindowSize', $event)"
      @config-reloaded="$emit('configReloaded')"
    />

    <!-- 更新弹窗 -->
    <UpdateModal
      v-model:show="showUpdateModal"
      :version-info="versionInfo"
    />
  </div>
</template>
