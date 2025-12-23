// 检查 WebSocket 连接状态
async function checkStatus() {
  const statusEl = document.getElementById('status')
  
  try {
    // 尝试连接测试
    const ws = new WebSocket('ws://127.0.0.1:9333')
    
    ws.onopen = () => {
      statusEl.className = 'status connected'
      statusEl.textContent = '✓ 已连接到 iterate'
      ws.close()
    }
    
    ws.onerror = () => {
      statusEl.className = 'status disconnected'
      statusEl.textContent = '✗ 未连接到 iterate'
    }
    
    // 3秒超时
    setTimeout(() => {
      if (ws.readyState !== WebSocket.OPEN) {
        statusEl.className = 'status disconnected'
        statusEl.textContent = '✗ 未连接到 iterate'
      }
    }, 3000)
  } catch (e) {
    statusEl.className = 'status disconnected'
    statusEl.textContent = '✗ 连接失败'
  }
}

// 页面加载时检查状态
checkStatus()
