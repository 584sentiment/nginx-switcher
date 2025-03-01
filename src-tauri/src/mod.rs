use serde::Serialize;
use std::fs;
use std::path::Path;

// 导入错误类型以保持一致性
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
}

// 获取hosts文件的系统路径
pub fn get_hosts_path() -> &'static Path {
    #[cfg(target_os = "windows")]
    {
        Path::new(r"C:\Windows\System32\drivers\etc\hosts")
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Path::new("/etc/hosts")
    }
}

// 检查是否有权限读取hosts文件
pub fn can_read_hosts() -> Result<bool, CommandError> {
    let hosts_path = get_hosts_path();

    match fs::metadata(hosts_path) {
        Ok(metadata) => Ok(metadata.permissions().readonly()),
        Err(e) => Err(CommandError {
            message: format!("Failed to check hosts file permissions: {}", e),
        }),
    }
}

// 检查是否有权限写入hosts文件
pub fn can_write_hosts() -> Result<bool, CommandError> {
    let hosts_path = get_hosts_path();
    let test_path = hosts_path.with_extension("test_write");

    // 尝试创建临时文件来测试写入权限
    match fs::write(&test_path, "test") {
        Ok(_) => {
            // 清理测试文件
            let _ = fs::remove_file(&test_path);
            Ok(true)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Ok(false)
            } else {
                Err(CommandError {
                    message: format!("Error testing write permissions: {}", e),
                })
            }
        }
    }
}

// 执行需要提权的hosts文件操作
pub async fn execute_with_privilege(
    operation: &str,
    args: Vec<String>,
    window: tauri::Window,
) -> Result<String, CommandError> {
    // 这里可以根据不同平台使用不同的提权方式
    #[cfg(target_os = "windows")]
    {
        use tauri::api::process::Command;

        // Windows通过UAC提权
        let mut command_args = vec![operation.to_string()];
        command_args.extend(args);

        Command::new("powershell")
            .args(&[
                "-Command",
                &format!(
                    "Start-Process -Verb RunAs -FilePath \"{}\" -ArgumentList \"{}\"",
                    std::env::current_exe().unwrap().to_str().unwrap(),
                    command_args.join(" ")
                ),
            ])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .map_err(|e| CommandError {
                message: format!("Failed to execute privileged operation: {}", e),
            })
    }

    #[cfg(target_os = "macos")]
    {
        // macOS上使用osascript提权
        use tauri::api::process::Command;

        let script = format!(
            "do shell script \"{}\" with administrator privileges",
            format!("{} {}", operation, args.join(" "))
        );

        Command::new("osascript")
            .args(&["-e", &script])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .map_err(|e| CommandError {
                message: format!("Failed to execute privileged operation: {}", e),
            })
    }

    #[cfg(target_os = "linux")]
    {
        // Linux上使用pkexec或sudo提权
        use tauri::api::process::Command;

        let mut command_args = vec![operation.to_string()];
        command_args.extend(args);

        // 尝试使用pkexec
        Command::new("pkexec")
            .args(&[&std::env::current_exe().unwrap().to_string_lossy()])
            .args(&command_args)
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .map_err(|e| CommandError {
                message: format!("Failed to execute privileged operation: {}", e),
            })
    }
}

// 特定的操作: 切换hosts文件IP状态
pub async fn toggle_ip_with_privilege(
    ip_address: String,
    window: tauri::Window,
) -> Result<bool, CommandError> {
    // 先检查是否有写权限
    if can_write_hosts()? {
        // 如果有权限，直接执行操作
        crate::toggle_host_ip_status(ip_address)
    } else {
        // 需要提权
        match execute_with_privilege("toggle_host_ip", vec![ip_address], window).await {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }
}

// 使用备用方法: 创建一个简单的提权通过tauri插件
#[cfg(feature = "with-tauri-plugin-authenticator")]
pub async fn toggle_ip_with_authenticator(
    ip_address: String,
    window: tauri::Window,
) -> Result<bool, CommandError> {
    use tauri_plugin_authenticator::AuthenticatorPlugin;

    // 这部分需要tauri-plugin-authenticator插件
    let result = AuthenticatorPlugin::request_authentication(
        &window,
        "需要管理员权限",
        &format!("修改hosts文件需要管理员权限，IP地址: {}", ip_address),
    )
    .await;

    match result {
        Ok(_) => {
            // 认证成功，执行操作
            crate::toggle_host_ip_status(ip_address)
        }
        Err(e) => Err(CommandError {
            message: format!("Authentication failed: {}", e),
        }),
    }
}
