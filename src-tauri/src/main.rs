use serde::Serialize;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use tauri_plugin_fs::FsExt;

#[derive(Debug, Serialize, Clone)]
pub struct HostEntry {
    ip: String,
    hostnames: Vec<String>,
    line: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct CommandError {
    message: String,
}

// 获取hosts文件的系统路径
#[tauri::command]
fn get_hosts_path() -> &'static Path {
    #[cfg(target_os = "windows")]
    {
        Path::new(r"C:\Windows\System32\drivers\etc\hosts")
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Path::new("/etc/hosts")
    }
}

// 解析 hosts 文件并返回有效条目
#[tauri::command]
fn get_hosts_entries() -> Result<Vec<HostEntry>, CommandError> {
    let hosts_path = get_hosts_path();
    let content = fs::read_to_string(hosts_path).map_err(|e| CommandError {
        message: format!("Failed to read hosts file: {}", e),
    })?;

    Ok(parse_hosts_content(content))
}

// 解析 hosts 文件内容
#[tauri::command]
fn parse_hosts_content(content: String) -> Vec<HostEntry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        // 保存原始行内容用于输出
        let original_line = line.to_string();

        // 检查行是否被注释
        let is_commented = line.trim_start().starts_with('#');
        let mut working_line = line.to_string();

        // 如果是注释行，移除开头的#并处理其后的内容
        if is_commented {
            // 找到第一个不是空白和#的字符的位置
            if let Some(first_char_pos) = line.find(|c| c != '#' && c != ' ' && c != '\t') {
                working_line = line[first_char_pos..].to_string();
            } else {
                // 如果整行都是#或空白，跳过
                continue;
            }
        } else {
            // 非注释行，但需要移除行内注释
            if let Some(idx) = line.find('#') {
                working_line = line[0..idx].to_string();
            }
        }

        // 去除前后空白
        let working_line = working_line.trim();

        // 跳过空行
        if working_line.is_empty() {
            continue;
        }

        // 分割IP和主机名
        let mut parts = working_line.split_whitespace();

        if let Some(ip_str) = parts.next() {
            // 验证是否是有效的IP地址
            if let Ok(_) = IpAddr::from_str(ip_str) {
                let mut hostnames = Vec::new();
                // 收集所有主机名
                for hostname in parts {
                    hostnames.push(hostname.to_string());
                }

                // 只有存在主机名时才添加条目
                if !hostnames.is_empty() {
                    entries.push(HostEntry {
                        ip: ip_str.to_string(),
                        hostnames,
                        line: original_line,
                        enabled: !is_commented,
                    });
                }
            }
        }
    }

    entries
}

// 获取原始 hosts 内容
#[tauri::command]
fn get_hosts_raw() -> Result<String, CommandError> {
    let hosts_path = get_hosts_path();

    fs::read_to_string(hosts_path).map_err(|e| CommandError {
        message: format!("Failed to read hosts file: {}", e),
    })
}

// 切换某个IP的状态
#[tauri::command]
fn toggle_host_ip_status(ip_address: String) -> Result<bool, CommandError> {
    let hosts_path = if cfg!(target_os = "windows") {
        Path::new(r"C:\Windows\System32\drivers\etc\hosts")
    } else {
        Path::new("/etc/hosts")
    };

    // 读取hosts文件
    let content = fs::read_to_string(hosts_path).map_err(|e| CommandError {
        message: format!("Failed to read hosts file: {}", e),
    })?;

    // 创建备份
    let backup_path = hosts_path.with_extension("bak");
    fs::write(&backup_path, &content).map_err(|e| CommandError {
        message: format!("Failed to create backup file: {}", e),
    })?;

    // 处理文件内容
    let mut new_content = String::new();
    let mut found = false;

    // 正则表达式匹配IP (包括可能被注释的情况)
    let ip_regex = regex::Regex::new(&format!(
        r"^(\s*)(#\s*)?({})(\s+.+)$",
        regex::escape(&ip_address)
    ))
    .map_err(|e| CommandError {
        message: format!("Failed to create regex: {}", e),
    })?;

    for line in content.lines() {
        if let Some(caps) = ip_regex.captures(line) {
            found = true;
            let leading_space = caps.get(1).map_or("", |m| m.as_str());
            let is_commented = caps.get(2).is_some();
            let matched_ip = caps.get(3).map_or("", |m| m.as_str());
            let rest_of_line = caps.get(4).map_or("", |m| m.as_str());

            if is_commented {
                // 当前是注释状态，取消注释
                new_content.push_str(&format!(
                    "{}{}{}\n",
                    leading_space, matched_ip, rest_of_line
                ));
            } else {
                // 当前是启用状态，添加注释
                new_content.push_str(&format!(
                    "{}# {}{}\n",
                    leading_space, matched_ip, rest_of_line
                ));
            }
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    // 如果没有找到匹配的IP，返回错误
    if !found {
        return Err(CommandError {
            message: format!("IP address {} not found in hosts file", ip_address),
        });
    }

    // 写回文件
    fs::write(hosts_path, new_content).map_err(|e| CommandError {
        message: format!("Failed to write hosts file: {}", e),
    })?;

    Ok(true)
}

#[tauri::command]
async fn modify_hosts_file(content: String) -> Result<(), String> {
    let hosts_path = if cfg!(target_os = "windows") {
        "C:\\Windows\\System32\\drivers\\etc\\hosts"
    } else {
        "/etc/hosts"
    };

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // 创建临时文件
        let temp_file = "/tmp/hosts_temp";
        std::fs::write(temp_file, content).map_err(|e| e.to_string())?;

        // 使用osascript请求管理员权限
        let script = format!(
            "do shell script \"cp {} {}\" with administrator privileges",
            temp_file, hosts_path
        );

        let output = Command::new("osascript")
            .args(&["-e", &script])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
    }

    // 其他平台的实现...

    Ok(())
}

// 扩展函数：获取特定IP地址的状态
#[tauri::command]
fn get_ip_status(ip_address: String) -> Result<bool, CommandError> {
    let hosts_entries = get_hosts_entries()?;

    for entry in hosts_entries {
        if entry.ip == ip_address {
            return Ok(entry.enabled);
        }
    }

    Err(CommandError {
        message: format!("IP address {} not found in hosts file", ip_address),
    })
}

// 在主函数中注册命令
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 定义hosts文件路径
            let hosts_path = get_hosts_path();

            // 注册自定义路径
            let _ = app.fs_scope().allow_directory(hosts_path, true);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_hosts_path,
            get_hosts_entries,
            get_hosts_raw,
            get_ip_status,
            parse_hosts_content,
            toggle_host_ip_status,
            modify_hosts_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
