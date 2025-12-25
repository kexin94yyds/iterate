// WebSocket 连接到 iterate 应用
let ws = null
let wsReconnectTimer = null
let heartbeatTimer = null
const WS_URL = 'ws://127.0.0.1:9333'

// 连接 WebSocket
function connectWebSocket() {
  if (ws && ws.readyState === WebSocket.OPEN) return

  try {
    ws = new WebSocket(WS_URL)

    ws.onopen = () => {
      console.log('[Iterate] WebSocket 已连接')
      clearReconnectTimer()
      // 立即发送一次心跳，让服务器知道这是浏览器扩展连接
      ws.send(JSON.stringify({ type: 'ping' }))
      startHeartbeat()
    }

    ws.onclose = () => {
      console.log('[Iterate] WebSocket 已断开')
      stopHeartbeat()
      scheduleReconnect()
    }

    ws.onerror = (error) => {
      console.log('[Iterate] WebSocket 错误:', error)
    }

    ws.onmessage = (event) => {
      console.log('[Iterate] 收到消息:', event.data)
      try {
        const data = JSON.parse(event.data)
        // 处理发送消息到 AI 的请求
        if (data.type === 'send_message') {
          sendMessageToAI(data.message, data.tabId)
        }
        // 处理获取 AI 回复的请求
        if (data.type === 'get_ai_response') {
          getAIResponse(data.tabId)
        }
      } catch (e) {
        console.log('[Iterate] 解析消息失败:', e)
      }
    }
  } catch (error) {
    console.log('[Iterate] WebSocket 连接失败:', error)
    scheduleReconnect()
  }
}

// 心跳保持连接
function startHeartbeat() {
  stopHeartbeat()
  heartbeatTimer = setInterval(() => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'ping' }))
    }
  }, 15000) // 每 15 秒发送心跳
}

function stopHeartbeat() {
  if (heartbeatTimer) {
    clearInterval(heartbeatTimer)
    heartbeatTimer = null
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
    console.log('[Iterate] aiResponse 长度:', message.data?.aiResponse?.length)

    // 发送到 iterate 应用
    const dataToSend = {
      type: 'ai_completed',
      ...message.data,
    }
    console.log('[Iterate] 发送数据:', JSON.stringify(dataToSend).substring(0, 200))
    const sent = sendToIterate(dataToSend)

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

// 发送消息到 AI 网站
async function sendMessageToAI(message, tabId) {
  console.log('[Iterate] 发送消息到 AI:', message, 'tabId:', tabId)
  
  // 如果指定了 tabId，发送到指定 tab
  if (tabId) {
    chrome.tabs.sendMessage(tabId, {
      type: 'INJECT_MESSAGE',
      message: message
    })
    return
  }
  
  // 否则发送到当前活动的 AI 网站 tab
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true })
  if (tabs.length > 0) {
    chrome.tabs.sendMessage(tabs[0].id, {
      type: 'INJECT_MESSAGE',
      message: message
    })
  }
}

// 获取 AI 回复
async function getAIResponse(tabId) {
  console.log('[Iterate] 获取 AI 回复, tabId:', tabId)

  let targetTabId = tabId
  if (!targetTabId) {
    const tabs = await chrome.tabs.query({ active: true, currentWindow: true })
    if (tabs.length > 0) {
      targetTabId = tabs[0].id
    }
  }

  if (!targetTabId) {
    console.log('[Iterate] 找不到目标 tab')
    return
  }

  try {
    const response = await chrome.tabs.sendMessage(targetTabId, { type: 'GET_AI_RESPONSE' })
    console.log('[Iterate] 获取到 AI 回复:', response)

    if (response?.success && response?.content && ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({
        type: 'ai_response',
        content: response.content,
        tabId: targetTabId,
      }))
    }
  }
  catch (e) {
    console.log('[Iterate] 获取 AI 回复失败:', e.message)
  }
}

// 扩展启动时连接 WebSocket
connectWebSocket()

// 定期检查连接状态
setInterval(() => {
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    connectWebSocket()
  }
}, 30000)
