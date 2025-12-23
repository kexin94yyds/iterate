<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useMessage } from 'naive-ui'
import { onMounted, onUnmounted, ref } from 'vue'

interface AiCompletionEvent {
  url: string
  title: string
  site_name: string
  message_preview: string
  timestamp: string
}

const message = useMessage()
const isMonitoring = ref(false)
const isConnecting = ref(false)
const completionEvents = ref<AiCompletionEvent[]>([])

let unlistenCompletion: (() => void) | null = null

async function startMonitoring() {
  isConnecting.value = true
  try {
    const result = await invoke('start_browser_monitoring', {})
    isMonitoring.value = true
    message.success(result as string)
  }
  catch (error: any) {
    message.error(`启动失败: ${error}`)
  }
  finally {
    isConnecting.value = false
  }
}

async function stopMonitoring() {
  try {
    await invoke('stop_browser_monitoring')
    isMonitoring.value = false
    message.info('浏览器监控已停止')
  }
  catch (error: any) {
    message.error(`停止失败: ${error}`)
  }
}

async function openUrl(url: string) {
  try {
    await invoke('open_browser_url', { url })
  }
  catch (error: any) {
    message.error(`打开 URL 失败: ${error}`)
  }
}

function clearEvents() {
  completionEvents.value = []
}

function formatTime(timestamp: string) {
  return new Date(timestamp).toLocaleTimeString()
}

onMounted(async () => {
  unlistenCompletion = await listen<AiCompletionEvent>('browser-ai-completed', (event) => {
    completionEvents.value.unshift(event.payload)
    if (completionEvents.value.length > 20) {
      completionEvents.value = completionEvents.value.slice(0, 20)
    }
    message.info(`${event.payload.site_name} AI 完成！`)
  })
})

onUnmounted(() => {
  if (unlistenCompletion) {
    unlistenCompletion()
  }
})
</script>

<template>
  <n-space vertical size="large">
    <!-- 说明 -->
    <div class="flex items-start">
      <div class="w-1.5 h-1.5 bg-info rounded-full mr-3 flex-shrink-0 mt-2" />
      <div>
        <div class="text-sm font-medium leading-relaxed mb-1">
          使用方式
        </div>
        <div class="text-xs opacity-60 leading-relaxed">
          1. 点击「开始监控」启动 WebSocket 服务器（端口 9333）<br>
          2. 安装 Chrome 扩展：<code class="bg-black-100 px-1 rounded">browser-extension</code> 目录<br>
          3. 打开 ChatGPT/Gemini 等网页，AI 完成后会收到通知
        </div>
      </div>
    </div>

    <!-- 控制按钮 -->
    <div class="flex items-center justify-between">
      <div class="flex items-center">
        <div class="w-1.5 h-1.5 rounded-full mr-3 flex-shrink-0" :class="isMonitoring ? 'bg-success' : 'bg-gray-400'" />
        <div>
          <div class="text-sm font-medium leading-relaxed">
            监控状态
          </div>
          <div class="text-xs opacity-60">
            {{ isMonitoring ? 'WebSocket 服务器运行中 (端口 9333)' : '未启动' }}
          </div>
        </div>
      </div>
      <n-space>
        <n-button
          v-if="!isMonitoring"
          size="small"
          type="primary"
          :loading="isConnecting"
          @click="startMonitoring"
        >
          开始监控
        </n-button>
        <n-button
          v-else
          size="small"
          type="error"
          @click="stopMonitoring"
        >
          停止监控
        </n-button>
      </n-space>
    </div>

    <!-- AI 完成通知列表 -->
    <div v-if="completionEvents.length > 0">
      <div class="flex items-center justify-between mb-2">
        <div class="text-sm font-medium">
          完成通知 ({{ completionEvents.length }})
        </div>
        <n-button size="tiny" text @click="clearEvents">
          清空
        </n-button>
      </div>
      <div class="space-y-2 max-h-48 overflow-y-auto">
        <div
          v-for="(event, index) in completionEvents"
          :key="index"
          class="p-2 bg-black-100 rounded cursor-pointer hover:bg-black-200"
          @click="openUrl(event.url)"
        >
          <div class="flex items-center justify-between mb-1">
            <span class="text-sm font-medium text-primary">
              {{ event.site_name }}
            </span>
            <span class="text-xs opacity-50">
              {{ formatTime(event.timestamp) }}
            </span>
          </div>
          <div class="text-xs opacity-60 truncate">
            {{ event.title }}
          </div>
          <div class="text-xs text-primary truncate mt-1">
            点击跳转 →
          </div>
        </div>
      </div>
    </div>
  </n-space>
</template>
