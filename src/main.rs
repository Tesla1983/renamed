use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::os::windows::io::AsRawHandle;
use windows_sys::Win32::System::Console::{GetConsoleMode, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING};
use chrono;
use rand::Rng;

/// 图片批量重命名工具
#[derive(Parser)]
#[command(name = "imgren")]
#[command(about = "批量重命名图片文件，支持多种命名格式")]
#[command(version)]
struct Args {
    /// 目标文件夹路径
    #[arg(long)]
    path: Option<String>,

    /// 文件名前缀
    #[arg(short = 'p', long)]
    prefix: Option<String>,

    /// 数字位数
    #[arg(short, long, default_value_t = 3)]
    digits: usize,

    /// 预设格式 (1-5)
    #[arg(short, long)]
    format: Option<String>,
}

#[derive(Debug, Clone)]
struct FormatConfig {
    pattern: String, // "NumberOnly", "PrefixNumber", "DateNumber", "OriginalNumber", 或自定义
    prefix: String,
    digits: usize,
    custom_format: bool,
}

fn main() {
    enable_ansi_support();
    println!("\x1b[92m========================================");
    println!("   \x1b[92m   图片批量重命名工具 ");
    println!("\x1b[92m========================================");

    let args = Args::parse();

    // 获取目标路径
    let target_path = match args.path {
        Some(p) => PathBuf::from(p),
        None => get_valid_path(),
    };

    // 切换工作目录
    std::env::set_current_dir(&target_path).expect("无法进入目录");
    println!(
        "\x1b[92m\n✓ 工作目录: {}",
        std::env::current_dir().unwrap().display()
    );

    // 扫描图片文件
    let image_exts = [
        ".jpg", ".jpeg", ".png", ".gif", ".webp", ".bmp", ".tiff", ".ico",
    ];
    let mut files: Vec<fs::DirEntry> = fs::read_dir(".")
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            if !path.is_file() {
                return false;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let ext_with_dot = format!(".{}", ext);
            let ext_ref = ext_with_dot.as_str();
            image_exts.iter().any(|ie| *ie == ext_ref)
        })
        .collect();

    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let total = files.len();
    if total == 0 {
        println!("\x1b[92m\n⚠️ 没有找到支持的图片文件！");
        println!("\x1b[92m\n当前目录文件：");
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.filter_map(|e| e.ok()).take(10) {
                let name = entry.file_name();
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let ext_lower = ext.to_lowercase();
                let ext_with_dot = format!(".{}", ext_lower);
                let ext_ref = ext_with_dot.as_str();
                let is_image = image_exts.iter().any(|ie| *ie == ext_ref);
                let icon = if is_image { "✓" } else { "✗" };
                println!("   {} {:?} (.{})", icon, name, ext);
            }
        }
        wait_for_key();
        return;
    }

    println!("\x1b[92m ✓ 找到 {} 个图片文件", total);

    // 选择命名格式
    let format_config = if let Some(fmt) = args.format {
        match fmt.as_str() {
            "1" => FormatConfig {
                pattern: "NumberOnly".to_string(),
                prefix: String::new(),
                digits: args.digits,
                custom_format: false,
            },
            "2" => FormatConfig {
                pattern: "PrefixNumber".to_string(),
                prefix: args.prefix.unwrap_or_else(|| "IMG".to_string()),
                digits: args.digits,
                custom_format: false,
            },
            _ => select_naming_format(args.digits),
        }
    } else {
        select_naming_format(args.digits)
    };

    // 显示预览
    show_preview(&files, &format_config, 5);

    // 确认执行
    print!("\x1b[92m\n========================================\n确认执行重命名？(y/n): ");
    io::stdout().flush().unwrap();

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).unwrap();

    if confirm.trim().to_lowercase() != "y" {
        println!("已取消");
        wait_for_key();
        return;
    }

    // 执行重命名
    println!("\x1b[92m\n开始处理...");
    let mut success = 0;
    let mut failed = 0;

    for (i, file) in files.iter().enumerate() {
        let old_path = file.path();
        let old_name =old_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        let ext = old_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();
        let ext_with_dot = format!(".{}", ext);

        let new_name = build_new_file_name(
            &format_config.pattern,
            &format_config.prefix,
            i + 1,
            format_config.digits,
            &ext_with_dot,
        );

        show_progress_bar(i + 1, total, &new_name);

        // 处理文件名冲突
        let final_name = if Path::new(&new_name).exists()
            && new_name != old_name 
        {
            let random_num: u16 = rand::thread_rng().gen_range(1000..10000);
            let base = Path::new(&new_name).file_stem().unwrap().to_str().unwrap_or("file");
            format!("{}_{}{}", base, random_num, ext_with_dot)
        } else {
            new_name.clone()
        };
        

        
        match fs::rename(&old_path, &final_name) {
            Ok(_) => success += 1,
            Err(e) => {
                failed += 1;
                eprintln!("\x1b[92m \n❌ 失败: {} -> {} - {}", old_name, final_name, e);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // 完成报告
    println!("\x1b[92m\n\n========================================");
    println!("\x1b[92m处理完成！");
    println!("\x1b[92m成功: {} 个", success);
    if failed > 0 {
        println!("\x1b[92m失败: {} 个", failed);
    }

    wait_for_key();
}

fn get_valid_path() -> PathBuf {
    loop {
        println!("\n========================================");
        println!("    \x1b[92m  图片批量重命名工具");
        println!("========================================");
        print!("\x1b[92m \n请输入文件夹路径（直接回车使用当前目录）: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        let path_str = if input.is_empty() { "." } else { input };

        let path = PathBuf::from(path_str);

        if path.is_dir() {
            return path.canonicalize().unwrap_or(path);
        }

        println!("\x1b[92m\n❌ 路径不存在: {}", path_str);
        println!("\x1b[92m请重新输入...");
    }
}

fn select_naming_format(default_digits: usize) -> FormatConfig {
    println!("\n========================================");
    println!(" \x1b[92m     选择重命名格式");
    println!("========================================");
    println!();
    println!(" \x1b[92m 1. 纯数字        示例: 001.jpg, 002.jpg ...");
    println!(" \x1b[92m 2. 前缀+数字     示例: IMG_001.jpg, IMG_002.jpg ...");
    println!(" \x1b[92m 3. 日期+数字     示例: 20240308_001.jpg ...");
    println!(" \x1b[92m 4. 原文件名+数字 示例: 照片_001.jpg, 照片_002.jpg ...");
    println!(" \x1b[92m 5. 自定义格式    输入你自己的格式 ...");
    println!();
    print!("\x1b[92m 请输入选项 (1-5，直接回车默认选1): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice = choice.trim();

    let choice = if choice.is_empty() { "1" } else { choice };

    let mut result = FormatConfig {
        pattern: String::new(),
        prefix: String::new(),
        digits: default_digits,
        custom_format: false,
    };

    match choice {
        "1" => {
            println!("\x1b[92m\n✓ 已选择: 纯数字格式");
            result.pattern = "NumberOnly".to_string();
        }
        "2" => {
            print!("\x1b[92m\n请输入前缀（直接回车使用默认'IMG'）: ");
            io::stdout().flush().unwrap();

            let mut prefix = String::new();
            io::stdin().read_line(&mut prefix).unwrap();

            result.prefix = if prefix.trim().is_empty() {
                "IMG".to_string()
            } else {
                prefix.trim().to_string()
            };
            result.pattern = "PrefixNumber".to_string();
            println!("\x1b[92m✓ 已选择: 前缀+数字格式，前缀='{}'", result.prefix);
        }
        "3" => {
            let date_str = chrono::Local::now().format("%Y%m%d").to_string();
            print!("\x1b[92m\n请输入日期（直接回车使用今天 {}）: ", date_str);
            io::stdout().flush().unwrap();

            let mut date_input = String::new();
            io::stdin().read_line(&mut date_input).unwrap();

            result.prefix = if date_input.trim().is_empty() {
                date_str
            } else {
                date_input.trim().to_string()
            };
            result.pattern = "DateNumber".to_string();
            println!("\x1b[92m✓ 已选择: 日期+数字格式，日期='{}'", result.prefix);
        }
        "4" => {
            print!("\x1b[92m\n请输入保留的文字（直接回车使用默认'照片'）: ");
            io::stdout().flush().unwrap();

            let mut name = String::new();
            io::stdin().read_line(&mut name).unwrap();

            result.prefix = if name.trim().is_empty() {
                " 照片".to_string()
            } else {
                name.trim().to_string()
            };
            result.pattern = "OriginalNumber".to_string();
            println!("\x1b[92m✓ 已选择: 保留名称+数字格式，名称='{}'", result.prefix);
        }
        "5" => {
            println!("\n========================================");
            println!("     \x1b[92m 自定义格式说明");
            println!("========================================");
            println!();
            println!("可用占位符:");
            println!(" \x1b[92m {{N}}  - 数字序号（如 001）");
            println!(" \x1b[92m {{P}}  - 你输入的前缀");
            println!(" \x1b[92m {{D}}  - 今天日期（YYYYMMDD）");
            println!(" \x1b[92m {{E}}  - 原文件扩展名（如 .jpg）");
            println!();
            println!("\x1b[92m示例:");
            println!(" \x1b[92m 输入: 旅行{{D}}_{{N}}");
            println!(" \x1b[92m 结果: 旅行20240308_001.jpg");
            println!();
            print!("\x1b[92m请输入你的格式: ");
            io::stdout().flush().unwrap();

            let mut custom = String::new();
            io::stdin().read_line(&mut custom).unwrap();
            let custom = custom.trim();

            if custom.is_empty() {
                println!("\x1b[92m格式为空，使用默认纯数字格式");
                result.pattern = "NumberOnly".to_string();
            } else {
                result.pattern = custom.to_string();
                result.custom_format = true;

                if custom.contains("{P}") {
                    print!("\x1b[92m请输入前缀值: ");
                    io::stdout().flush().unwrap();
                    let mut prefix_val = String::new();
                    io::stdin().read_line(&mut prefix_val).unwrap();
                    result.prefix = prefix_val.trim().to_string();
                }

                println!("\x1b[92m✓ 已选择: 自定义格式 '{}'", custom);
            }
        }
        _ => {
            println!("\x1b[92m\n⚠️ 无效选项，使用默认纯数字格式");
            result.pattern = "NumberOnly".to_string();
        }
    }

    // 询问数字位数
    print!("\x1b[92m\n请输入数字位数（2-6，直接回车默认3位）: ");
    io::stdout().flush().unwrap();

    let mut digits_input = String::new();
    io::stdin().read_line(&mut digits_input).unwrap();

    if let Ok(d) = digits_input.trim().parse::<usize>() {
        if d >= 2 && d <= 6 {
            result.digits = d;
        } else {
            println!("\x1b[92m位数超出范围，使用默认3位");
        }
    } else if !digits_input.trim().is_empty() {
        println!("\x1b[92m输入无效，使用默认3位");
    }

    println!("\x1b[92m✓ 数字位数: {} 位", result.digits);

    result
}

fn build_new_file_name(
    pattern: &str,
    prefix: &str,
    number: usize,
    digits: usize,
    extension: &str,
) -> String {
    let num_str = format!("{:0width$}", number, width = digits);
    let date_str = chrono::Local::now().format("%Y%m%d").to_string();

    match pattern {
        "NumberOnly" => format!("{}{}", num_str, extension),
        "PrefixNumber" => format!("{}_{}{}", prefix, num_str, extension),
        "DateNumber" => format!("{}_{}{}", prefix, num_str, extension),
        "OriginalNumber" => format!("{}_{}{}", prefix, num_str, extension),
        _ => {
            // 自定义格式
            let mut name = pattern.to_string();
            name = name.replace("{N}", &num_str);
            name = name.replace("{P}", prefix);
            name = name.replace("{D}", &date_str);
            format!("{}{}", name, extension)
        }
    }
}

fn show_preview(files: &[fs::DirEntry], config: &FormatConfig, count: usize) {
    println!("\n========================================");
    println!("      重命名预览（前 {} 个）", count);
    println!("========================================");
    println!();

    for (i, file) in files.iter().take(count).enumerate() {
        let ext = file
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();

        let ext_with_dot = format!(".{}", ext);
        let new_name = build_new_file_name(
            &config.pattern,
            &config.prefix,
            i + 1,
            config.digits,
            &ext_with_dot,
        );

        println!("  {}", file.file_name().to_str().unwrap());
        println!("    ↓");
        println!("  {}", new_name);
        println!();
    }

    if files.len() > count {
        println!("... 还有 {} 个文件 ...", files.len() - count);
    }
}

fn enable_ansi_support() {
    unsafe {
        let handle = io::stdout().as_raw_handle() as isize;
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
        
    }
}

fn show_progress_bar(current: usize, total: usize, status: &str) {
    let percent = (current as f64 / total as f64 * 100.0) as usize;
    let filled = (current as f64 / total as f64 * 30.0) as usize;
    let empty = 30 - filled;

    print!("\r[");

    // 绿色填充部分
    for _ in 0..filled {
        print!("\x1b[92m█\x1b[0m");
    }

    // 灰色空白部分
    for _ in 0..empty {
        print!("\x1b[90m░\x1b[0m");
    }

    print!("] ");
    print!("\x1b[92m{}% ({}/{})\x1b[0m ", percent, current, total);
    print!("\x1b[92m{}\x1b[0m", status);

    io::stdout().flush().unwrap();
    if current == total {
        println!();
        
    }
}

fn wait_for_key() {
    println!("\x1b[92m\n按回车键退出...");
    let mut _pause = String::new();
    io::stdin().read_line(&mut _pause).unwrap();
}
