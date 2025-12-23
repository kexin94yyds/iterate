// 监控 AI 生成状态 - 通过检测 Stop 按钮或加载状态
const AI_SITES = {
  'chat.openai.com': { name: 'ChatGPT', stopSelector: 'button[aria-label="Stop generating"], button[data-testid="stop-button"], button[aria-label="Stop"]' },
  'chatgpt.com': { name: 'ChatGPT', stopSelector: 'button[aria-label="Stop generating"], button[data-testid="stop-button"], button[aria-label="Stop"]' },
  'gemini.google.com': { name: 'Gemini', stopSelector: 'button[aria-label="Stop response"], button[aria-label="Stop"], mat-icon[data-mat-icon-name="stop_circle"], .stop-button' },
  'aistudio.google.com': { name: 'AI Studio', stopSelector: 'button[aria-label="Stop"], mat-icon[data-mat-icon-name="stop"]' },
  'claude.ai': { name: 'Claude', stopSelector: 'button[aria-label="Stop Response"], button[aria-label="Stop"]' },
  'poe.com': { name: 'Poe', stopSelector: 'button[class*="StopButton"], button[class*="stop"]' },
  'notebooklm.google.com': { name: 'NotebookLM', stopSelector: 'button[aria-label="Stop"], .stop-button' },
  'www.perplexity.ai': { name: 'Perplexity', stopSelector: 'button[aria-label="Stop"], button[class*="stop"]' },
  'perplexity.ai': { name: 'Perplexity', stopSelector: 'button[aria-label="Stop"], button[class*="stop"]' },
  'chat.deepseek.com': { name: 'DeepSeek', stopSelector: 'button[aria-label="Stop"], .stop-btn, button[class*="stop"]' },
  'grok.x.ai': { name: 'Grok', stopSelector: 'button[aria-label="Stop"], button[class*="stop"]' },
  'x.com': { name: 'Grok', stopSelector: 'button[aria-label="Stop"], button[class*="stop"]' },
  'www.genspark.ai': { name: 'Genspark', stopSelector: 'button[aria-label="Stop"], button[class*="stop"]' },
  'tongyi.aliyun.com': { name: '通义千问', stopSelector: 'button[aria-label="停止"], button[class*="stop"], .stop-btn' },
  'www.doubao.com': { name: '豆包', stopSelector: 'button[aria-label="停止"], button[class*="stop"]' },
  'ima.qq.com': { name: 'IMA', stopSelector: 'button[aria-label="停止"], button[class*="stop"]' },
  'kimi.moonshot.cn': { name: 'Kimi', stopSelector: 'button[aria-label="停止"], button[class*="stop"], .stop-btn' },
  'yuanbao.tencent.com': { name: '腾讯元宝', stopSelector: 'button[aria-label="停止"], button[class*="stop"]' },
}

const config = AI_SITES[window.location.hostname]

if (!config) {
  console.log('[iterate] 不支持的网站')
} else {
  console.log(`[iterate] 开始监控 ${config.name}`)
  
  let wasGenerating = false
  
  function isGenerating() {
    const stopBtn = document.querySelector(config.stopSelector)
    return stopBtn && stopBtn.offsetParent !== null
  }
  
  function sendNotification() {
    console.log('[iterate] ✅ AI 完成！发送通知...')
    chrome.runtime.sendMessage({
      type: 'AI_COMPLETED',
      data: {
        siteName: config.name,
        url: window.location.href,
        title: document.title,
        timestamp: new Date().toISOString(),
      }
    })
  }
  
  // 定时检测
  setInterval(() => {
    const generating = isGenerating()
    
    if (generating && !wasGenerating) {
      console.log('[iterate] 🔄 AI 开始生成...')
    }
    
    if (wasGenerating && !generating) {
      sendNotification()
    }
    
    wasGenerating = generating
  }, 500)
  
  console.log('[iterate] 监控已启动，等待 AI 生成...')
}
