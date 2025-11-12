#![cfg_attr(windows, windows_subsystem = "windows")]

mod converter;
mod parser;
mod types;

use anyhow::{Context, Result};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(windows)]
use winapi::um::consoleapi::AllocConsole;
#[cfg(windows)]
use winapi::um::wincon::{GetConsoleWindow};
#[cfg(windows)]
use winapi::um::winuser::{MessageBoxW, MB_ICONINFORMATION, MB_ICONERROR, MB_OK};

#[cfg(windows)]
fn allocate_console() -> bool {
    unsafe {
        // 检查是否已有控制台
        if !GetConsoleWindow().is_null() {
            return true;
        }
        // 分配新控制台
        AllocConsole() != 0
    }
}

#[cfg(windows)]
fn show_message_box(title: &str, message: &str, is_error: bool) {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    let title_wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(once(0))
        .collect();
    let message_wide: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(once(0))
        .collect();

    unsafe {
        let icon = if is_error { MB_ICONERROR } else { MB_ICONINFORMATION };
        MessageBoxW(
            std::ptr::null_mut(),
            message_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_OK | icon,
        );
    }
}

#[cfg(not(windows))]
fn allocate_console() -> bool {
    true
}

#[cfg(not(windows))]
fn free_console() {}

#[cfg(not(windows))]
fn show_message_box(title: &str, message: &str, _is_error: bool) {
    println!("{}: {}", title, message);
}

/// 格式化数字为带千位分隔符的字符串
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
        count += 1;
    }

    result.chars().rev().collect()
}

fn main() {
    if let Err(e) = run() {
        let error_msg = format!("转换过程中发生错误：\n\n{}", e);
        show_message_box("错误", &error_msg, true);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // 检查是否有参数
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    // 获取 exe 所在目录
    let exe_dir = get_exe_dir()?;

    let mut all_output_paths = Vec::new();
    let mut total_files = 0;

    // 收集所有有效的文件和文件夹
    let mut valid_files = Vec::new();
    let mut valid_folders = Vec::new();

    for arg in &args[1..] {
        let input_path = PathBuf::from(arg);

        if !input_path.exists() {
            println!("警告: 路径不存在，跳过 - {}", input_path.display());
            continue;
        }

        if input_path.is_file() {
            let ext = input_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            if ext == "xdts" || ext == "tdts" {
                valid_files.push(input_path);
            }
        } else if input_path.is_dir() {
            valid_folders.push(input_path);
        }
    }

    // 判断是否为单文件模式（只有1个文件，没有文件夹）
    let is_single_file_mode = valid_files.len() == 1 && valid_folders.is_empty();

    // 多文件/文件夹模式：分配控制台显示进度
    if !is_single_file_mode {
        if !allocate_console() {
            // 无法分配控制台，改用消息框
            show_message_box(
                "错误",
                "无法创建控制台窗口",
                true,
            );
            return Ok(());
        }
    }

    // 处理单独拖放的文件
    if !valid_files.is_empty() {
        if !is_single_file_mode {
            println!("\n{}", "=".repeat(60));
            println!("处理 {} 个文件", valid_files.len());
            println!("{}", "=".repeat(60));
        }

        for (idx, input_path) in valid_files.iter().enumerate() {
            if !is_single_file_mode {
                println!(
                    "\n[{}/{}] {}",
                    idx + 1,
                    valid_files.len(),
                    input_path.file_name().unwrap().to_string_lossy()
                );
                println!("{}", "-".repeat(60));
            }

            match process_file(&input_path, None, false, is_single_file_mode) {
                Ok(output_paths) => {
                    all_output_paths.extend(output_paths.clone());
                    total_files += 1;
                    if !is_single_file_mode && valid_files.len() > 1 {
                        println!("✓ 完成 ({} 个 STS 文件)", output_paths.len());
                    }
                }
                Err(e) => {
                    if !is_single_file_mode {
                        eprintln!("✗ 转换失败: {}", e);
                    }
                }
            }
        }
    }

    // 处理拖放的文件夹
    for (folder_idx, input_path) in valid_folders.iter().enumerate() {
        println!("\n{}", "=".repeat(60));
        if valid_folders.len() > 1 {
            println!(
                "[文件夹 {}/{}] 扫描: {}",
                folder_idx + 1,
                valid_folders.len(),
                input_path.display()
            );
        } else {
            println!("扫描文件夹: {}", input_path.display());
        }
        println!("{}", "=".repeat(60));

        let timesheet_files = find_timesheet_files(&input_path)?;

        if timesheet_files.is_empty() {
            println!("未找到 .xdts 或 .tdts 文件");
            continue;
        }

        println!("找到 {} 个文件:", timesheet_files.len());
        for f in &timesheet_files {
            println!("  - {}", f.file_name().unwrap().to_string_lossy());
        }
        println!();

        // 在 exe 同目录下创建 converted_sts 文件夹
        let output_dir = exe_dir.join("converted_sts");
        std::fs::create_dir_all(&output_dir)
            .context("无法创建输出目录")?;

        // 转换每个文件
        for (idx, ts_file) in timesheet_files.iter().enumerate() {
            println!("{}", "-".repeat(60));
            println!(
                "[{}/{}] 正在处理: {}",
                idx + 1,
                timesheet_files.len(),
                ts_file.file_name().unwrap().to_string_lossy()
            );

            match process_file(&ts_file, Some(&output_dir), false, false) {
                Ok(output_paths) => {
                    all_output_paths.extend(output_paths.clone());
                    total_files += 1;
                    println!("✓ 完成 ({} 个 STS 文件)", output_paths.len());
                }
                Err(e) => {
                    eprintln!("✗ 转换失败: {}", e);
                }
            }
        }
    }

    // 单文件模式：用消息框显示结果
    if is_single_file_mode {
        if total_files > 0 && !all_output_paths.is_empty() {
            for path in &all_output_paths {
                let _size = std::fs::metadata(path)?.len();
                let _file_name = path.file_name().unwrap().to_string_lossy();
            }
        } else {
            show_message_box("转换失败", "文件转换失败，请检查文件格式。", true);
        }
        return Ok(());
    }

    // 多文件/文件夹模式：显示详细总结
    println!("\n{}", "=".repeat(60));
    println!("转换完成!");
    println!("{}", "=".repeat(60));
    println!("处理了 {} 个源文件", total_files);
    println!("生成了 {} 个 STS 文件", all_output_paths.len());

    if !all_output_paths.is_empty() {
        println!("\n生成的文件:");
        for path in all_output_paths.iter().take(10) {
            let size = std::fs::metadata(path)?.len();
            println!(
                "  - {} ({} 字节)",
                path.file_name().unwrap().to_string_lossy(),
                format_number(size)
            );
        }
        if all_output_paths.len() > 10 {
            println!("  ... 还有 {} 个文件", all_output_paths.len() - 10);
        }
    }

    println!("\n按任意键退出...");
    let _ = io::stdin().read_line(&mut String::new());

    Ok(())
}

fn process_file(
    input_path: &Path,
    output_dir: Option<&Path>,
    verbose: bool,
    quiet: bool,
) -> Result<Vec<PathBuf>> {
    // 加载时间表
    if !verbose && !quiet {
        println!("正在加载: {}", input_path.display());
    }

    let timesheets = parser::load_timesheets(input_path)?;

    if !verbose && !quiet {
        println!("找到 {} 个时间表", timesheets.len());
    }

    // 确定输出目录
    let output_dir = match output_dir {
        Some(dir) => dir.to_path_buf(),
        None => input_path
            .parent()
            .context("无法获取父目录")?
            .to_path_buf(),
    };

    let mut output_paths = Vec::new();

    // 转换每个时间表
    for (i, ts) in timesheets.iter().enumerate() {
        // 生成输出文件名
        let output_name = if timesheets.len() == 1 {
            format!(
                "{}.sts",
                input_path.file_stem().unwrap().to_string_lossy()
            )
        } else {
            let safe_name = ts
                .name
                .replace('/', "_")
                .replace('\\', "_")
                .replace(':', "_");
            let safe_name = if safe_name.len() > 100 {
                &safe_name[..100]
            } else {
                &safe_name
            };
            format!(
                "{}_{:03}_{}.sts",
                input_path.file_stem().unwrap().to_string_lossy(),
                i,
                safe_name
            )
        };

        let output_path = output_dir.join(output_name);

        // 转换并保存
        match converter::save_sts(ts, &output_path, verbose) {
            Ok(_) => {
                output_paths.push(output_path.clone());
                if !verbose && !quiet {
                    println!(
                        "✓ 已转换: {}",
                        output_path.file_name().unwrap().to_string_lossy()
                    );
                }
            }
            Err(e) => {
                if !quiet {
                    eprintln!("✗ 转换失败: {}", ts.name);
                    eprintln!("  错误: {}", e);
                }
            }
        }
    }

    Ok(output_paths)
}

fn find_timesheet_files(folder_path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(folder_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if ext_lower == "xdts" || ext_lower == "tdts" {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

fn get_exe_dir() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("无法获取程序路径")?;
    exe_path
        .parent()
        .map(|p| p.to_path_buf())
        .context("无法获取程序目录")
}

fn print_usage() {
    let usage_msg = "XDTS/TDTS 转 STS 转换工具\n\n\
        使用方法：\n\n\
        1. 拖放单个 .xdts/.tdts 文件到本程序\n\
           → 转换到文件同目录\n\n\
        2. 拖放文件夹到本程序\n\
           → 查找并转换文件夹内所有 xdts/tdts 文件\n\
           → 保存到 'converted_sts' 目录中\n\n\
           ";

    show_message_box("使用说明", usage_msg, false);
}
