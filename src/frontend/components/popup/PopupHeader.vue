<script setup lang="ts">
import { computed } from 'vue'
import ThemeIcon from '../common/ThemeIcon.vue'

interface Props {
  currentTheme?: string
  loading?: boolean
  showMainLayout?: boolean
  alwaysOnTop?: boolean
  projectPath?: string
  linkUrl?: string
  linkTitle?: string
}

interface Emits {
  themeChange: [theme: string]
  openMainLayout: []
  toggleAlwaysOnTop: []
  newChat: []
}

const props = withDefaults(defineProps<Props>(), {
  currentTheme: 'dark',
  loading: false,
  showMainLayout: false,
  alwaysOnTop: false,
  projectPath: undefined,
})

const emit = defineEmits<Emits>()

// 显示完整的项目路径
const displayProjectPath = computed(() => {
  return props.projectPath || null
})

function handleThemeChange() {
  // 切换到下一个主题
  const nextTheme = props.currentTheme === 'light' ? 'dark' : 'light'
  emit('themeChange', nextTheme)
}

function handleOpenMainLayout() {
  emit('openMainLayout')
}

function handleToggleAlwaysOnTop() {
  emit('toggleAlwaysOnTop')
}

function handleNewChat() {
  emit('newChat')
}
</script>

<template>
  <div class="px-4 py-3 select-none">
    <div class="flex items-center justify-between">
      <!-- 左侧：标题和项目路径/链接 -->
      <div class="flex items-center gap-3 min-w-0 flex-1">
        <span class="text-lg font-bold text-primary-500 flex-shrink-0">∞</span>
        <h1 class="text-base font-medium text-white flex-shrink-0">
          iterate
        </h1>
        <!-- 链接标题 (cmd+点击打开) -->
        <a
          v-if="props.linkUrl"
          :href="props.linkUrl"
          target="_blank"
          class="text-sm text-primary-400 hover:text-primary-300 truncate cursor-pointer"
          :title="`${props.linkTitle || props.linkUrl}\n(Cmd+点击打开)`"
        >
          {{ props.linkTitle || props.linkUrl }}
        </a>
        <span v-else-if="displayProjectPath" class="text-sm text-gray-400 truncate" :title="displayProjectPath">
          / {{ displayProjectPath }}
        </span>
      </div>

      <!-- 右侧：操作按钮 -->
      <n-space size="small">
        <!-- 新聊天按钮 -->
        <n-button
          size="small"
          quaternary
          circle
          title="打开新聊天窗口"
          @click="handleNewChat"
        >
          <template #icon>
            <div class="i-carbon-add w-4 h-4 text-white" />
          </template>
        </n-button>
        <!-- 置顶按钮 -->
        <n-button
          size="small"
          quaternary
          circle
          :title="props.alwaysOnTop ? '取消置顶' : '窗口置顶'"
          @click="handleToggleAlwaysOnTop"
        >
          <template #icon>
            <div
              :class="props.alwaysOnTop ? 'i-carbon-pin-filled' : 'i-carbon-pin'"
              class="w-4 h-4 text-white"
            />
          </template>
        </n-button>
        <n-button
          size="small"
          quaternary
          circle
          :title="props.showMainLayout ? '返回聊天' : '打开设置'"
          @click="handleOpenMainLayout"
        >
          <template #icon>
            <div
              :class="props.showMainLayout ? 'i-carbon-chat' : 'i-carbon-settings'"
              class="w-4 h-4 text-white"
            />
          </template>
        </n-button>
        <n-button
          size="small"
          quaternary
          circle
          :title="`切换到${props.currentTheme === 'light' ? '深色' : '浅色'}主题`"
          @click="handleThemeChange"
        >
          <template #icon>
            <ThemeIcon :theme="props.currentTheme" class="w-4 h-4 text-white" />
          </template>
        </n-button>
      </n-space>
    </div>
  </div>
</template>
