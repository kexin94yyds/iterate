<script setup lang="ts">
import type { McpRequest } from '../../types/popup'
import hljs from 'highlight.js'
import MarkdownIt from 'markdown-it'
import { useMessage } from 'naive-ui'
import { computed, nextTick, onMounted, onUpdated, ref, watch } from 'vue'

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  currentTheme: 'dark',
})

const emit = defineEmits<Emits>()

// 预处理引用内容，移除增强prompt格式标记
function preprocessQuoteContent(content: string): string {
  let processedContent = content

  // 定义需要移除的格式标记
  const markersToRemove = [
    /### BEGIN RESPONSE ###\s*/gi,
    /Here is an enhanced version of the original instruction that is more specific and clear:\s*/gi,
    /<augment-enhanced-prompt>\s*/gi,
    /<\/augment-enhanced-prompt>\s*/gi,
    /### END RESPONSE ###\s*/gi,
  ]

  // 逐个移除格式标记
  markersToRemove.forEach((marker) => {
    processedContent = processedContent.replace(marker, '')
  })

  // 清理多余的空行和首尾空白
  processedContent = processedContent
    .replace(/\n\s*\n\s*\n/g, '\n\n') // 将多个连续空行合并为两个
    .trim() // 移除首尾空白

  return processedContent
}

// 动态导入代码高亮样式，根据主题切换

// 动态加载代码高亮样式
function loadHighlightStyle(theme: string) {
  // 移除现有的highlight.js样式
  const existingStyle = document.querySelector('link[data-highlight-theme]')
  if (existingStyle) {
    existingStyle.remove()
  }

  // 根据主题选择样式
  const styleName = theme === 'light' ? 'github' : 'github-dark'

  // 动态创建样式链接
  const link = document.createElement('link')
  link.rel = 'stylesheet'
  link.href = `https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/${styleName}.min.css`
  link.setAttribute('data-highlight-theme', theme)
  document.head.appendChild(link)
}

interface Props {
  request: McpRequest | null
  loading?: boolean
  currentTheme?: string
  browserAiResponse?: string | null
}

interface Emits {
  quoteMessage: [message: string]
  toggleSendTarget: [target: 'ide' | 'browser']
}

// 发送目标切换状态（从 localStorage 读取持久化设置）
const SEND_TARGET_KEY = 'iterate_send_target'
const savedTarget = localStorage.getItem(SEND_TARGET_KEY) as 'ide' | 'browser' | null
const sendTarget = ref<'ide' | 'browser'>(savedTarget || 'ide')

// 本地的浏览器 AI 回复状态（直接从 props.request 获取）
const localBrowserAiResponse = computed(() => props.request?.browser_ai_response || null)

function setSendTarget(target: 'ide' | 'browser') {
  sendTarget.value = target
  localStorage.setItem(SEND_TARGET_KEY, target)
  emit('toggleSendTarget', target)
}

// 初始化时通知父组件当前状态
onMounted(() => {
  if (savedTarget) {
    emit('toggleSendTarget', savedTarget)
  }
})

const message = useMessage()

// 复制原文到剪贴板
async function copyMessage() {
  // Web 模式下优先复制 AI 回复内容
  if (sendTarget.value === 'browser' && localBrowserAiResponse.value) {
    try {
      await navigator.clipboard.writeText(localBrowserAiResponse.value)
      message.success('AI 回复已复制到剪贴板')
    }
    catch {
      message.error('复制失败')
    }
  }
  else if (props.request?.message) {
    // IDE 模式下复制原消息
    try {
      const processedContent = preprocessQuoteContent(props.request.message)
      await navigator.clipboard.writeText(processedContent)
      message.success('原文已复制到剪贴板')
    }
    catch {
      message.error('复制失败')
    }
  }
}

// 引用消息内容
function quoteMessage() {
  // Web 模式下优先引用 AI 回复内容
  if (sendTarget.value === 'browser' && localBrowserAiResponse.value) {
    emit('quoteMessage', localBrowserAiResponse.value)
  }
  else if (props.request?.message) {
    // IDE 模式下引用原消息
    const processedContent = preprocessQuoteContent(props.request.message)
    emit('quoteMessage', processedContent)
  }
}

// 创建 Markdown 实例 - 保持代码高亮功能
const md = new MarkdownIt({
  html: true,
  xhtmlOut: false,
  breaks: true,
  langPrefix: 'language-',
  linkify: true,
  typographer: true,
  quotes: '""\'\'',
  highlight(str: string, lang: string) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(str, { language: lang }).value
      }
      catch {
        // 忽略错误
      }
    }
    return ''
  },
})

// 自定义链接渲染器 - 移除外链的点击功能，保持视觉样式
md.renderer.rules.link_open = function (tokens, idx, options, env, renderer) {
  const token = tokens[idx]
  const href = token.attrGet('href')

  // 检查是否是外部链接
  if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
    // 移除href属性，保持其他属性
    token.attrSet('href', '#')
    token.attrSet('onclick', 'return false;')
    token.attrSet('style', 'cursor: default; text-decoration: none;')
    token.attrSet('title', `外部链接已禁用: ${href}`)
  }

  return renderer.renderToken(tokens, idx, options)
}

// 自定义自动链接渲染器 - 处理linkify生成的链接
md.renderer.rules.autolink_open = function (tokens, idx, options, env, renderer) {
  const token = tokens[idx]
  const href = token.attrGet('href')

  // 检查是否是外部链接
  if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
    // 移除href属性，保持其他属性
    token.attrSet('href', '#')
    token.attrSet('onclick', 'return false;')
    token.attrSet('style', 'cursor: default; text-decoration: none;')
    token.attrSet('title', `外部链接已禁用: ${href}`)
  }

  return renderer.renderToken(tokens, idx, options)
}

// Markdown 渲染函数
function renderMarkdown(content: string) {
  try {
    // 将字面量 \n 转换为实际换行符（AI 有时会发送转义的换行符）
    const normalizedContent = content.replace(/\\n/g, '\n')
    return md.render(normalizedContent)
  }
  catch (error) {
    console.error('Markdown 渲染失败:', error)
    return content
  }
}

// 创建复制按钮
function createCopyButton(preEl: Element) {
  // 检查是否已经有复制按钮
  if (preEl.querySelector('.copy-button'))
    return

  const copyButton = document.createElement('div')
  copyButton.className = 'copy-button'
  // 极简设计：无背景，无边框
  copyButton.style.cssText = `
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 1000;
    opacity: 1;
    transition: opacity 0.2s ease;
    pointer-events: auto;
    height: 20px;
    width: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
  `

  copyButton.innerHTML = `
    <button style="
      display: flex;
      align-items: center;
      justify-content: center;
      width: 100%;
      height: 100%;
      color: #9ca3af;
      transition: color 0.2s ease;
      border: none;
      background: none;
      cursor: pointer;
      padding: 0;
      margin: 0;
    " onmouseover="this.style.color='#14b8a6'" onmouseout="this.style.color='#9ca3af'">
      <div class="i-carbon-copy" style="width: 16px; height: 16px; display: block;"></div>
    </button>
  `

  const button = copyButton.querySelector('button')!
  button.addEventListener('click', async (e) => {
    e.stopPropagation()
    e.preventDefault()
    try {
      const codeEl = preEl.querySelector('code')
      const textContent = codeEl?.textContent || preEl.textContent || ''
      await navigator.clipboard.writeText(textContent)

      // 更新为成功状态
      const icon = button.querySelector('div')!
      icon.className = 'i-carbon-checkmark'
      icon.style.cssText = 'width: 16px; height: 16px; color: #22c55e; display: block;'

      setTimeout(() => {
        icon.className = 'i-carbon-copy'
        icon.style.cssText = 'width: 16px; height: 16px; display: block;'
      }, 2000)
      message.success('代码已复制到剪贴板')
    }
    catch {
      message.error('复制失败')
    }
  })

  // 确保父元素有相对定位和足够的层级
  const preElement = preEl as HTMLElement
  preElement.style.position = 'relative'
  preElement.style.zIndex = '1'

  // 按钮始终显示，不需要悬停事件

  preElement.appendChild(copyButton)
}

// 设置内联代码复制
function setupInlineCodeCopy() {
  const inlineCodeElements = document.querySelectorAll('.markdown-content p code, .markdown-content li code')
  inlineCodeElements.forEach((codeEl) => {
    codeEl.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(codeEl.textContent || '')
        message.success('代码已复制到剪贴板')
      }
      catch {
        message.error('复制失败')
      }
    })
  })
}

// 设置代码复制功能
let setupCodeCopyTimer: number | null = null
function setupCodeCopy() {
  if (setupCodeCopyTimer) {
    clearTimeout(setupCodeCopyTimer)
  }

  // 增加延迟时间，确保DOM完全渲染
  setupCodeCopyTimer = window.setTimeout(() => {
    nextTick(() => {
      // 确保选择正确的 pre 元素
      const preElements = document.querySelectorAll('.markdown-content pre')
      console.log('设置代码复制按钮，找到', preElements.length, '个代码块')
      preElements.forEach((preEl) => {
        createCopyButton(preEl)
      })
      setupInlineCodeCopy()

      // 如果没有找到代码块，再次尝试
      if (preElements.length === 0) {
        setTimeout(() => {
          const retryElements = document.querySelectorAll('.markdown-content pre')
          console.log('重试设置代码复制按钮，找到', retryElements.length, '个代码块')
          retryElements.forEach((preEl) => {
            createCopyButton(preEl)
          })
        }, 200)
      }
    })
  }, 300)
}

// 监听request变化，重新设置代码复制
watch(() => props.request, () => {
  if (props.request) {
    setupCodeCopy()
  }
}, { deep: true })

// 监听loading状态变化
watch(() => props.loading, (newLoading) => {
  if (!newLoading && props.request) {
    setupCodeCopy()
  }
})

onMounted(() => {
  // 初始化代码高亮样式
  loadHighlightStyle(props.currentTheme)
  if (props.request) {
    setupCodeCopy()
  }
})

// 监听主题变化
watch(() => props.currentTheme, (newTheme) => {
  loadHighlightStyle(newTheme)
}, { immediate: false })

// 在DOM更新后也尝试设置
onUpdated(() => {
  if (props.request && !props.loading) {
    setupCodeCopy()
  }
})
</script>

<template>
  <div class="text-white">
    <!-- 加载状态 -->
    <div v-if="loading" class="flex flex-col items-center justify-center py-8">
      <n-spin size="medium" />
      <p class="text-sm mt-3 text-white opacity-60">
        加载中...
      </p>
    </div>

    <!-- 消息显示区域 -->
    <div v-else-if="request?.message" class="relative">
      <!-- 主要内容 -->
      <div
        v-if="request.is_markdown"
        class="markdown-content prose prose-sm max-w-none prose-headings:font-semibold prose-headings:leading-tight prose-h1:!mt-4 prose-h1:!mb-2 prose-h1:!text-lg prose-h1:!font-bold prose-h1:!leading-tight prose-h2:!mt-3 prose-h2:!mb-1.5 prose-h2:!text-base prose-h2:!font-semibold prose-h2:!leading-tight prose-h3:!mt-2.5 prose-h3:!mb-1 prose-h3:!text-sm prose-h3:!font-medium prose-h3:!leading-tight prose-h4:!mt-2 prose-h4:!mb-1 prose-h4:!text-sm prose-h4:!font-medium prose-h4:!leading-tight prose-p:my-1 prose-p:leading-relaxed prose-p:text-sm prose-ul:my-1 prose-ul:text-sm prose-ul:pl-4 prose-ol:my-1 prose-ol:text-sm prose-ol:pl-4 prose-li:my-1 prose-li:text-sm prose-li:leading-relaxed prose-blockquote:my-2 prose-blockquote:text-sm prose-blockquote:pl-4 prose-blockquote:ml-0 prose-blockquote:italic prose-blockquote:border-l-4 prose-blockquote:border-primary-500 prose-pre:relative prose-pre:border prose-pre:rounded-lg prose-pre:p-4 prose-pre:my-3 prose-pre:overflow-x-auto scrollbar-code prose-code:px-1 prose-code:py-0.5 prose-code:text-xs prose-code:cursor-pointer prose-code:font-mono prose-a:text-primary-500 prose-a:no-underline prose-a:cursor-default [&_a[onclick='return false;']]:opacity-60 [&_a[onclick='return false;']]:cursor-not-allowed" :class="[
          currentTheme === 'light' ? 'prose-slate' : 'prose-invert',
          currentTheme === 'light' ? 'prose-headings:text-gray-900' : 'prose-headings:text-white',
          currentTheme === 'light' ? 'prose-p:text-gray-700' : 'prose-p:text-white prose-p:opacity-85',
          currentTheme === 'light' ? 'prose-ul:text-gray-700 prose-ol:text-gray-700 prose-li:text-gray-700' : 'prose-ul:text-white prose-ul:opacity-85 prose-ol:text-white prose-ol:opacity-85 prose-li:text-white prose-li:opacity-85',
          currentTheme === 'light' ? 'prose-blockquote:text-gray-600' : 'prose-blockquote:text-gray-300 prose-blockquote:opacity-90',
          currentTheme === 'light' ? 'prose-pre:bg-gray-50 prose-pre:border-gray-200' : 'prose-pre:bg-black prose-pre:border-gray-700',
          currentTheme === 'light' ? 'prose-strong:text-gray-900 prose-strong:font-semibold' : 'prose-strong:text-white prose-strong:font-semibold',
          currentTheme === 'light' ? 'prose-em:text-gray-600 prose-em:italic' : 'prose-em:text-gray-300 prose-em:italic',
        ]"
        v-html="renderMarkdown(request.message)"
      />
      <div v-else class="whitespace-pre-wrap leading-relaxed text-white">
        {{ request.message }}
      </div>

      <!-- 浏览器 AI 回复（Web 模式下显示） -->
      <div v-if="localBrowserAiResponse" class="mt-4 pt-4 border-t border-gray-600/30">
        <div
          class="markdown-content prose prose-sm max-w-none prose-headings:font-semibold prose-headings:leading-tight prose-h1:!mt-4 prose-h1:!mb-2 prose-h1:!text-lg prose-h1:!font-bold prose-h1:!leading-tight prose-h2:!mt-3 prose-h2:!mb-1.5 prose-h2:!text-base prose-h2:!font-semibold prose-h2:!leading-tight prose-h3:!mt-2.5 prose-h3:!mb-1 prose-h3:!text-sm prose-h3:!font-medium prose-h3:!leading-tight prose-h4:!mt-2 prose-h4:!mb-1 prose-h4:!text-sm prose-h4:!font-medium prose-h4:!leading-tight prose-p:my-1 prose-p:leading-relaxed prose-p:text-sm prose-ul:my-1 prose-ul:text-sm prose-ul:pl-4 prose-ol:my-1 prose-ol:text-sm prose-ol:pl-4 prose-li:my-1 prose-li:text-sm prose-li:leading-relaxed prose-blockquote:my-2 prose-blockquote:text-sm prose-blockquote:pl-4 prose-blockquote:ml-0 prose-blockquote:italic prose-blockquote:border-l-4 prose-blockquote:border-primary-500 prose-pre:relative prose-pre:border prose-pre:rounded-lg prose-pre:p-4 prose-pre:my-3 prose-pre:overflow-x-auto scrollbar-code prose-code:px-1 prose-code:py-0.5 prose-code:text-xs prose-code:cursor-pointer prose-code:font-mono prose-a:text-primary-500 prose-a:no-underline prose-a:cursor-default" :class="[
            currentTheme === 'light' ? 'prose-slate' : 'prose-invert',
            currentTheme === 'light' ? 'prose-headings:text-gray-900' : 'prose-headings:text-white',
            currentTheme === 'light' ? 'prose-p:text-gray-700' : 'prose-p:text-white prose-p:opacity-85',
            currentTheme === 'light' ? 'prose-ul:text-gray-700 prose-ol:text-gray-700 prose-li:text-gray-700' : 'prose-ul:text-white prose-ul:opacity-85 prose-ol:text-white prose-ol:opacity-85 prose-li:text-white prose-li:opacity-85',
            currentTheme === 'light' ? 'prose-blockquote:text-gray-600' : 'prose-blockquote:text-gray-300 prose-blockquote:opacity-90',
            currentTheme === 'light' ? 'prose-pre:bg-gray-50 prose-pre:border-gray-200' : 'prose-pre:bg-black prose-pre:border-gray-700',
            currentTheme === 'light' ? 'prose-strong:text-gray-900 prose-strong:font-semibold' : 'prose-strong:text-white prose-strong:font-semibold',
            currentTheme === 'light' ? 'prose-em:text-gray-600 prose-em:italic' : 'prose-em:text-gray-300 prose-em:italic',
          ]"
          v-html="renderMarkdown(localBrowserAiResponse)"
        />
      </div>

      <!-- 操作按钮区域 -->
      <div class="flex justify-between items-center mt-4 pt-3 border-t border-gray-600/30" data-guide="quote-message">
        <!-- 左侧：发送目标切换（两个按钮） -->
        <div class="inline-flex rounded-md overflow-hidden border border-gray-500/30 bg-black-200">
          <div
            title="发送到 IDE"
            class="custom-recessed-button flex items-center gap-1 px-3 py-1.5 text-xs font-medium cursor-pointer transition-all duration-100"
            :class="sendTarget === 'ide' ? 'is-active text-black' : 'bg-transparent text-gray-400 hover:text-gray-200'"
            @click="setSendTarget('ide')"
          >
            <div class="i-carbon-terminal w-3.5 h-3.5" />
            <span>IDE</span>
          </div>
          <div
            title="发送到浏览器 AI"
            class="custom-recessed-button flex items-center gap-1 px-3 py-1.5 text-xs font-medium cursor-pointer transition-all duration-100"
            :class="sendTarget === 'browser' ? 'is-active text-black' : 'bg-transparent text-gray-400 hover:text-gray-200'"
            @click="setSendTarget('browser')"
          >
            <div class="i-carbon-globe w-3.5 h-3.5" />
            <span>Web</span>
          </div>
        </div>

        <!-- 右侧：复制和引用按钮 -->
        <div class="flex gap-2">
          <div
            title="点击复制AI的消息内容到剪贴板"
            class="custom-recessed-button inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-gray-300 hover:text-white rounded-md cursor-pointer border border-gray-500/30 bg-gray-800/50"
            @click="copyMessage"
          >
            <div class="i-carbon-copy w-3.5 h-3.5" />
            <span>复制原文</span>
          </div>
          <div
            title="点击将AI的消息内容引用到输入框中"
            class="custom-recessed-button inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-gray-300 hover:text-white rounded-md cursor-pointer border border-gray-500/30 bg-gray-800/50"
            @click="quoteMessage"
          >
            <div class="i-carbon-quotes w-3.5 h-3.5" />
            <span>引用原文</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 错误状态 -->
    <n-alert v-else type="error" title="数据加载错误">
      <div class="text-white">
        Request对象: {{ JSON.stringify(request) }}
      </div>
    </n-alert>
  </div>
</template>
