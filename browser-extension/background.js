// WebSocket 连接到 iterate 应用
let ws = null
let wsReconnectTimer = null
const WS_URL = 'ws://127.0.0.1:9333'

// 连接 WebSocket
function connectWebSocket() {
  if (ws && ws.readyState === WebSocket.OPEN) return

  try {
    ws = new WebSocket(WS_URL)

    ws.onopen = () => {
      console.log('[Iterate] WebSocket 已连接')
      clearReconnectTimer()
    }

    ws.onclose = () => {
      console.log('[Iterate] WebSocket 已断开')
      scheduleReconnect()
    }

    ws.onerror = (error) => {
      console.log('[Iterate] WebSocket 错误:', error)
    }

    ws.onmessage = (event) => {
      console.log('[Iterate] 收到消息:', event.data)
    }
  } catch (error) {
    console.log('[Iterate] WebSocket 连接失败:', error)
    scheduleReconnect()
  }
}

// 清除重连定时器
function clearReconnectTimer() {
  if (wsReconnectTimer) {
    clearTimeout(wsReconnectTimer)
    wsReconnectTimer = null
  }
}

// 安排重连
function scheduleReconnect() {
  clearReconnectTimer()
  wsReconnectTimer = setTimeout(() => {
    console.log('[Iterate] 尝试重连...')
    connectWebSocket()
  }, 5000)
}

// 发送消息到 iterate
function sendToIterate(data) {
  console.log('[iterate] sendToIterate 调用, ws状态:', ws ? ws.readyState : 'null')
  if (ws && ws.readyState === WebSocket.OPEN) {
    const msg = JSON.stringify(data)
    console.log('[iterate] 发送消息:', msg)
    ws.send(msg)
    return true
  }
  console.log('[iterate] WebSocket 未连接，无法发送')
  return false
}

// 监听来自 content script 的消息
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'AI_COMPLETED') {
    console.log('[Iterate] AI 完成:', message.data)

    // 发送到 iterate 应用
    const sent = sendToIterate({
      type: 'ai_completed',
      ...message.data,
    })

    // 如果 WebSocket 未连接，显示浏览器通知
    if (!sent) {
      chrome.notifications.create({
        type: 'basic',
        iconUrl: 'icon128.png',
        title: `${message.data.siteName} AI 完成`,
        message: message.data.title || '点击查看',
        priority: 2,
      })
    }

    sendResponse({ success: true })
  }
  return true
})

// 点击通知时打开对应标签页
chrome.notifications.onClicked.addListener((notificationId) => {
  // 可以在这里实现跳转到对应标签页的逻辑
  console.log('[Iterate] 通知被点击:', notificationId)
})

// 扩展启动时连接 WebSocket
connectWebSocket()

// 定期检查连接状态
setInterval(() => {
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    connectWebSocket()
  }
}, 30000)
