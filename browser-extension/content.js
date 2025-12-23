// iterate - AI 完成监控
// 支持：文本生成、图片生成、代码执行等

const AI_SITES = {
  'chat.openai.com': { name: 'ChatGPT', type: 'chat' },
  'chatgpt.com': { name: 'ChatGPT', type: 'chat' },
  'gemini.google.com': { name: 'Gemini', type: 'chat' },
  'aistudio.google.com': { name: 'AI Studio', type: 'studio' },
  'claude.ai': { name: 'Claude', type: 'chat' },
  'poe.com': { name: 'Poe', type: 'chat' },
  'notebooklm.google.com': { name: 'NotebookLM', type: 'chat' },
  'www.perplexity.ai': { name: 'Perplexity', type: 'chat' },
  'perplexity.ai': { name: 'Perplexity', type: 'chat' },
  'chat.deepseek.com': { name: 'DeepSeek', type: 'chat' },
  'grok.x.ai': { name: 'Grok', type: 'chat' },
  'x.com': { name: 'Grok', type: 'chat' },
  'www.genspark.ai': { name: 'Genspark', type: 'chat' },
  'tongyi.aliyun.com': { name: '通义千问', type: 'chat' },
  'www.doubao.com': { name: '豆包', type: 'chat' },
  'ima.qq.com': { name: 'IMA', type: 'chat' },
  'kimi.moonshot.cn': { name: 'Kimi', type: 'chat' },
  'yuanbao.tencent.com': { name: '腾讯元宝', type: 'chat' },
}

// Stop 按钮选择器
const STOP_SELECTORS = {
  'chat.openai.com': 'button[aria-label="Stop generating"], button[data-testid="stop-button"], button[aria-label="Stop"]',
  'chatgpt.com': 'button[aria-label="Stop generating"], button[data-testid="stop-button"], button[aria-label="Stop"]',
  'gemini.google.com': 'button[aria-label="Stop response"], button[aria-label="Stop"], mat-icon[data-mat-icon-name="stop_circle"]',
  'aistudio.google.com': 'button[aria-label="Stop"], button[aria-label="Cancel"]',
  'claude.ai': 'button[aria-label="Stop Response"], button[aria-label="Stop"]',
  'poe.com': 'button[class*="StopButton"], button[class*="stop"]',
  'notebooklm.google.com': 'button[aria-label="Stop"], .stop-button',
  'www.perplexity.ai': 'button[aria-label="Stop"], button[class*="stop"]',
  'perplexity.ai': 'button[aria-label="Stop"], button[class*="stop"]',
  'chat.deepseek.com': 'button[aria-label="Stop"], .stop-btn',
  'grok.x.ai': 'button[aria-label="Stop"]',
  'x.com': 'button[aria-label="Stop"]',
  'www.genspark.ai': 'button[aria-label="Stop"]',
  'tongyi.aliyun.com': 'button[aria-label="停止"], .stop-btn',
  'www.doubao.com': 'button[aria-label="停止"]',
  'ima.qq.com': 'button[aria-label="停止"]',
  'kimi.moonshot.cn': 'button[aria-label="停止"], .stop-btn',
  'yuanbao.tencent.com': 'button[aria-label="停止"]',
}

const hostname = window.location.hostname
const config = AI_SITES[hostname]

if (!config) {
  console.log('[iterate] 不支持的网站:', hostname)
} else {
  console.log(`[iterate] 开始监控 ${config.name}`)

  let state = {
    wasGenerating: false,
    wasRunning: false,
    lastRunTime: null,
    imageCount: 0,
  }

  // 检测 Stop 按钮
  function hasStopButton() {
    const selector = STOP_SELECTORS[hostname]
    if (!selector) return false
    const btn = document.querySelector(selector)
    return btn && btn.offsetParent !== null
  }

  // 检测运行状态文本 (AI Studio 特有)
  function detectRunningText() {
    const pageText = document.body.innerText
    const runningPatterns = ['Running', 'Generating', 'Thinking', '正在运行', '生成中', '思考中']
    return runningPatterns.some(p => pageText.includes(p))
  }

  // 检测完成状态 (AI Studio: "Ran for Xs")
  function detectCompletionText() {
    const pageText = document.body.innerText
    const ranMatch = pageText.match(/Ran for (\d+)s/)
    if (ranMatch) {
      return { completed: true, runTime: parseInt(ranMatch[1]) }
    }
    const thoughtMatch = pageText.match(/Thought for (\d+) seconds/)
    if (thoughtMatch) {
      return { completed: true, thinkTime: parseInt(thoughtMatch[1]) }
    }
    return { completed: false }
  }

  // 检测新生成的图片
  function countGeneratedImages() {
    const imgs = document.querySelectorAll('img[src*="generated"], img[src*="output"], img[src*="blob:"], img[src*="data:image"]')
    return imgs.length
  }

  // 检测加载指示器
  function hasLoadingIndicator() {
    const spinners = document.querySelectorAll('[role="progressbar"], .loading, .spinner, [class*="loading"], [class*="spinner"]')
    return Array.from(spinners).some(s => s.offsetParent !== null)
  }

  // 综合判断是否在生成中
  function isGenerating() {
    // 优先检测 Stop 按钮
    if (hasStopButton()) return true
    // AI Studio 特殊处理
    if (config.type === 'studio') {
      if (detectRunningText()) return true
      if (hasLoadingIndicator()) return true
    }
    return false
  }

  // 发送通知
  function sendNotification(extra = {}) {
    // 检查 chrome.runtime 是否可用（扩展刷新后可能失效）
    if (!chrome?.runtime?.sendMessage) {
      console.log('[iterate] ⚠️ 扩展上下文已失效，请刷新页面')
      return
    }
    
    const message = {
      type: 'AI_COMPLETED',
      data: {
        siteName: config.name,
        url: window.location.href,
        title: document.title,
        timestamp: new Date().toISOString(),
        ...extra,
      },
    }
    console.log('[iterate] ✅ AI 完成! 发送通知...', extra)
    
    try {
      chrome.runtime.sendMessage(message)
    } catch (e) {
      console.log('[iterate] ⚠️ 发送消息失败，请刷新页面:', e.message)
    }
  }

  // 主检测循环
  setInterval(() => {
    const generating = isGenerating()
    const completion = detectCompletionText()
    const currentImageCount = countGeneratedImages()

    // 检测生成开始
    if (generating && !state.wasGenerating) {
      console.log('[iterate] 🔄 AI 开始生成...')
      state.imageCount = currentImageCount
    }

    // 检测生成完成
    if (state.wasGenerating && !generating) {
      const extra = {}
      if (completion.runTime) extra.runTime = completion.runTime
      if (completion.thinkTime) extra.thinkTime = completion.thinkTime
      sendNotification(extra)
    }

    // 检测新图片生成完成 (AI Studio 图片生成)
    if (config.type === 'studio' && currentImageCount > state.imageCount && !generating) {
      console.log('[iterate] 🖼️ 检测到新图片!')
      sendNotification({ imageGenerated: true, newImages: currentImageCount - state.imageCount })
      state.imageCount = currentImageCount
    }

    state.wasGenerating = generating
  }, 500)

  console.log('[iterate] 监控已启动，等待 AI 生成...')

  // 监听来自 background 的消息注入请求
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.type === 'INJECT_MESSAGE') {
      console.log('[iterate] 📝 收到消息注入请求:', message.message)
      injectMessageToInput(message.message)
      sendResponse({ success: true })
    }
    return true
  })

  // 注入消息到 AI 输入框
  function injectMessageToInput(text) {
    const inputSelectors = {
      'chatgpt.com': 'textarea[data-id="root"], #prompt-textarea, textarea[placeholder*="Message"]',
      'chat.openai.com': 'textarea[data-id="root"], #prompt-textarea, textarea[placeholder*="Message"]',
      'gemini.google.com': 'div[contenteditable="true"], textarea',
      'aistudio.google.com': 'textarea, div[contenteditable="true"]',
      'claude.ai': 'div[contenteditable="true"], textarea',
      'chat.deepseek.com': 'textarea',
      'kimi.moonshot.cn': 'textarea',
      'tongyi.aliyun.com': 'textarea',
      'www.doubao.com': 'textarea',
    }

    const selector = inputSelectors[hostname] || 'textarea, div[contenteditable="true"]'
    const input = document.querySelector(selector)

    if (!input) {
      console.log('[iterate] ⚠️ 找不到输入框')
      return false
    }

    // 根据输入框类型填入内容
    if (input.tagName === 'TEXTAREA') {
      input.value = text
      input.dispatchEvent(new Event('input', { bubbles: true }))
    } else if (input.contentEditable === 'true') {
      input.textContent = text
      input.dispatchEvent(new InputEvent('input', { bubbles: true }))
    }

    // 聚焦输入框
    input.focus()
    console.log('[iterate] ✅ 消息已注入到输入框')

    // 延迟后自动点击发送按钮
    setTimeout(() => {
      clickSendButton()
    }, 200)

    return true
  }

  // 点击发送按钮
  function clickSendButton() {
    const sendButtonSelectors = {
      'chatgpt.com': 'button[data-testid="send-button"], button[aria-label*="Send"], button[data-tooltip*="发送"], form button:last-child:not([disabled]):not([data-testid="composer-plus-btn"])',
      'chat.openai.com': 'button[data-testid="send-button"], button[aria-label*="Send"], button[data-tooltip*="发送"], form button:last-child:not([disabled]):not([data-testid="composer-plus-btn"])',
      'gemini.google.com': 'button[aria-label*="Send"], button.send-button, [data-mat-icon-name="send"]',
      'aistudio.google.com': 'button[aria-label*="Send"], button.send-button',
      'claude.ai': 'button[aria-label*="Send"], button[type="submit"]',
      'chat.deepseek.com': 'button[type="submit"], .send-btn',
      'kimi.moonshot.cn': 'button[type="submit"]',
      'tongyi.aliyun.com': 'button[type="submit"]',
      'www.doubao.com': 'button[type="submit"]',
    }

    const selector = sendButtonSelectors[hostname] || 'button[type="submit"], button[aria-label*="Send"]'
    const buttons = document.querySelectorAll(selector)

    console.log('[iterate] 🔍 找到按钮数量:', buttons.length, '选择器:', selector)
    buttons.forEach((btn, i) => {
      console.log(`[iterate] 按钮 ${i}:`, btn, 'disabled:', btn.disabled, 'visible:', btn.offsetParent !== null)
    })

    // 找到可点击的发送按钮（优先选择真正的发送按钮）
    const validButtons = Array.from(buttons).filter(btn => !btn.disabled && btn.offsetParent !== null)

    // 优先选择有 send-button/submit 相关属性的按钮
    const sendBtn = validButtons.find(btn =>
      btn.dataset.testid?.includes('send')
      || btn.id?.includes('submit')
      || btn.ariaLabel?.includes('发送')
      || btn.ariaLabel?.toLowerCase().includes('send'),
    ) || validButtons[0]

    if (sendBtn) {
      console.log('[iterate] 🚀 点击发送按钮', sendBtn)

      // 模拟完整的鼠标事件序列（React 需要）
      const mousedownEvent = new MouseEvent('mousedown', { bubbles: true, cancelable: true, view: window })
      const mouseupEvent = new MouseEvent('mouseup', { bubbles: true, cancelable: true, view: window })
      const clickEvent = new MouseEvent('click', { bubbles: true, cancelable: true, view: window })

      sendBtn.dispatchEvent(mousedownEvent)
      sendBtn.dispatchEvent(mouseupEvent)
      sendBtn.dispatchEvent(clickEvent)
      return true
    }

    // 备用方案：用 Enter 键发送
    console.log('[iterate] ⚠️ 找不到发送按钮，尝试 Enter 键发送')
    return sendWithEnterKey()
  }

  // 使用 Enter 键发送消息
  function sendWithEnterKey() {
    const inputSelectors = {
      'chatgpt.com': 'textarea[data-id="root"], #prompt-textarea, textarea[placeholder*="Message"]',
      'chat.openai.com': 'textarea[data-id="root"], #prompt-textarea, textarea[placeholder*="Message"]',
    }
    const selector = inputSelectors[hostname] || 'textarea, div[contenteditable="true"]'
    const input = document.querySelector(selector)

    if (input) {
      input.focus()
      const enterEvent = new KeyboardEvent('keydown', {
        key: 'Enter',
        code: 'Enter',
        keyCode: 13,
        which: 13,
        bubbles: true,
        cancelable: true,
      })
      input.dispatchEvent(enterEvent)
      console.log('[iterate] ⌨️ 已发送 Enter 键')
      return true
    }
    return false
  }

  // 获取最新的 AI 回复内容
  function getLatestAIResponse() {
    const responseSelectors = {
      'chatgpt.com': '[data-message-author-role="assistant"] .markdown',
      'chat.openai.com': '[data-message-author-role="assistant"] .markdown',
      'gemini.google.com': '.model-response-text, .response-content',
      'aistudio.google.com': '.response-container, .model-response',
      'claude.ai': '[data-testid="assistant-message"], .assistant-message',
      'chat.deepseek.com': '.assistant-message, .ai-response',
      'kimi.moonshot.cn': '.assistant-message',
      'tongyi.aliyun.com': '.assistant-message',
      'www.doubao.com': '.assistant-message',
    }

    const selector = responseSelectors[hostname] || '.assistant-message, .ai-response, .model-response'
    const responses = document.querySelectorAll(selector)

    if (responses.length === 0) {
      console.log('[iterate] ⚠️ 找不到 AI 回复')
      return null
    }

    // 获取最后一个回复
    const lastResponse = responses[responses.length - 1]
    const content = lastResponse.textContent
    console.log('[iterate] 📖 获取到 AI 回复，长度:', content?.length)
    return content?.trim() || null
  }

  // 监听来自 background 的获取回复请求
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.type === 'GET_AI_RESPONSE') {
      console.log('[iterate] 📝 收到获取 AI 回复请求')
      const response = getLatestAIResponse()
      sendResponse({ success: !!response, content: response })
    }
    return true
  })
}
