use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process;
use crate::log_important;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowInstance {
    pub pid: u32,
    pub project_path: String,
    pub window_title: String,
    pub registered_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WindowRegistry {
    pub instances: Vec<WindowInstance>,
}

impl WindowRegistry {
    fn registry_path() -> PathBuf {
        let tmp_dir = std::env::temp_dir();
        tmp_dir.join("iterate_windows.json")
    }

    pub fn load() -> Self {
        let path = Self::registry_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(registry) => return registry,
                        Err(e) => {
                            log_important!(warn, "解析窗口注册表失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log_important!(warn, "读取窗口注册表失败: {}", e);
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::registry_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化窗口注册表失败: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("写入窗口注册表失败: {}", e))?;
        Ok(())
    }

    pub fn register(&mut self, project_path: &str) -> Result<(), String> {
        let pid = process::id();
        
        // 清理已失效的实例
        self.cleanup_stale_instances();
        
        // 检查是否已注册
        if self.instances.iter().any(|i| i.pid == pid) {
            // 更新项目路径
            if let Some(instance) = self.instances.iter_mut().find(|i| i.pid == pid) {
                instance.project_path = project_path.to_string();
                instance.window_title = format!("iterate — {}", project_path);
            }
        } else {
            // 新增注册
            self.instances.push(WindowInstance {
                pid,
                project_path: project_path.to_string(),
                window_title: format!("iterate — {}", project_path),
                registered_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        
        self.save()?;
        log_important!(info, "窗口已注册: PID={}, 项目={}", pid, project_path);
        Ok(())
    }

    pub fn unregister(&mut self) -> Result<(), String> {
        let pid = process::id();
        self.instances.retain(|i| i.pid != pid);
        self.save()?;
        log_important!(info, "窗口已注销: PID={}", pid);
        Ok(())
    }

    pub fn get_all_instances(&mut self) -> Vec<WindowInstance> {
        self.cleanup_stale_instances();
        let _ = self.save();
        self.instances.clone()
    }

    fn cleanup_stale_instances(&mut self) {
        self.instances.retain(|instance| {
            is_process_running(instance.pid)
        });
    }
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
            })
            .unwrap_or(false)
    }
    
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

pub fn activate_window(pid: u32) -> Result<(), String> {
    log_important!(info, "[DEBUG] activate_window 被调用，目标 PID: {}", pid);
    log_important!(info, "[DEBUG] 当前进程 PID: {}", process::id());

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        log_important!(info, "[DEBUG] 开始执行 macOS 窗口激活");
        
        // 方法1: 使用 open 命令通过 bundle ID 和 PID 激活
        // 先尝试使用 kill -0 确认进程存在
        let check = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        
        if check.is_err() || !check.unwrap().status.success() {
            return Err(format!("进程 {} 不存在", pid));
        }
        
        // 使用 AppleScript 通过 process id 激活（更精确）
        let script = format!(
            r#"
            tell application "System Events"
                set allProcs to every process whose unix id is {}
                if (count of allProcs) > 0 then
                    set targetProc to item 1 of allProcs
                    
                    -- 取消最小化所有窗口
                    tell targetProc
                        repeat with w in windows
                            try
                                set miniaturized of w to false
                            end try
                        end repeat
                    end tell
                    
                    -- 使用 AXRaise 激活窗口
                    tell targetProc
                        try
                            perform action "AXRaise" of window 1
                        end try
                    end tell
                    
                    -- 设置为前台
                    set frontmost of targetProc to true
                end if
            end tell
            "#,
            pid
        );
        
        log_important!(info, "[DEBUG] 执行 AppleScript，目标 PID: {}", pid);
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("激活窗口失败: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_important!(warn, "[DEBUG] AppleScript 警告: {}", stderr);
            
            // 备用方案: 使用 open 命令
            log_important!(info, "[DEBUG] 尝试备用方案: open 命令");
            let _ = Command::new("open")
                .args(["-a", "iterate"])
                .output();
        } else {
            log_important!(info, "[DEBUG] AppleScript 执行成功");
        }
        
        Ok(())
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Err("暂不支持此平台的窗口激活".to_string())
    }
}
