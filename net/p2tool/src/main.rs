use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::io::{self, Write, Read};
use std::time::{Instant, Duration};
use std::process::exit;

struct Args {
    url: String,
    limit: String,
    out: PathBuf,
    nogui: bool,
    raw: bool,
}

fn parse_args() -> Args {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Console::*;
        use windows_sys::Win32::Foundation::HANDLE;
        unsafe {
            let stdout: HANDLE = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode: u32 = 0;
            if GetConsoleMode(stdout, &mut mode) != 0 {
                SetConsoleMode(stdout, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }

    let mut url = None;
    let mut limit = "infinity".to_string();
    let mut out = PathBuf::from(".");
    let mut nogui = false;
    let mut raw = false;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        for &(short, desc) in &[("i", "-i"), ("l", "-l"), ("o", "-o"), ("ng", "-ng"), ("r", "-r")] {
            if arg.starts_with("--") && &arg[2..] == short {
                eprintln!("\x1b[33mWarning: short flag '{}' used with double dash '--{}'. Correct usage: '{}'\x1b[0m", desc, short, desc);
                exit(1);
            }
        }

        match arg.as_str() {
            "--install" | "-i" => url = iter.next(),
            "--limit"   | "-l" => limit = iter.next().unwrap_or("infinity".to_string()),
            "--output"  | "-o" => out = PathBuf::from(iter.next().unwrap_or(".".to_string())),
            "--nogui"   | "-ng" => nogui = true,
            "-raw" => raw = true,
            _ => { eprintln!("\x1b[33mWarning: Unknown argument: {}\x1b[0m", arg); exit(1); }
        }
    }

    let url = url.unwrap_or_else(|| { eprintln!("\x1b[31mError: --install is required\x1b[0m"); exit(1); });

    Args { url, limit, out, nogui, raw }
}

fn github_to_jsdelivr(url: &str) -> Option<String> {
    let s = url.trim();
    let s = if s.starts_with("http://") || s.starts_with("https://") { s.to_string() } else { format!("https://{}", s) };
    if s.contains("cdn.jsdelivr.net/gh/") || s.contains("raw.githubusercontent.com/") { return None; }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() < 7 || parts[2] != "github.com" || parts[5] != "blob" { return None; }

    let user = parts[3]; let repo = parts[4]; let branch = parts[6];
    let path = if parts.len() > 7 { parts[7..].join("/") } else { String::new() };
    Some(if path.is_empty() {
        format!("https://cdn.jsdelivr.net/gh/{}/{}@{}/", user, repo, branch)
    } else {
        format!("https://cdn.jsdelivr.net/gh/{}/{}@{}/{}", user, repo, branch, path)
    })
}

fn fname(url: &str) -> String { url.rsplit('/').next().unwrap_or("file").to_string() }

fn lim_str(limit: &str) -> String {
    let s = limit.trim().to_uppercase();
    if s.is_empty() || s == "INFINITY" || s == "-1" { "∞ B/s".to_string() }
    else { format!("{}{}", &s[..s.len()-1], match s.chars().last().unwrap() {'K'=>" KB/s",'M'=>" MB/s",'G'=>" GB/s",'B'=>" B/s", _=>" B/s"}) }
}

fn eta(sec: f64) -> String {
    if sec < 60.0 { format!("{}s", sec.round() as u64) }
    else if sec < 3600.0 { format!("{}m", (sec/60.0).round() as u64) }
    else { format!("{}h", (sec/3600.0).round() as u64) }
}

#[cfg(target_os = "windows")]
fn hide() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
    unsafe { let hwnd = GetConsoleWindow(); if !hwnd.is_null() { ShowWindow(hwnd, 0); } }
}

fn download(url: &str, out: &PathBuf, limit: &str) -> io::Result<()> {
    let resp = ureq::get(url).call().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(out)?;
    let rate: Option<u64> = match limit.trim().to_uppercase().as_str() {
        "INFINITY" | "-1" | "" => None,
        s if s.ends_with('K') => Some(s[..s.len()-1].parse::<u64>().unwrap_or(1)*1024),
        s if s.ends_with('M') => Some(s[..s.len()-1].parse::<u64>().unwrap_or(1)*1024*1024),
        s if s.ends_with('G') => Some(s[..s.len()-1].parse::<u64>().unwrap_or(1)*1024*1024*1024),
        s => Some(s.parse().unwrap_or(0)),
    };

    let mut buf=[0u8;1024]; let mut downloaded=0u64; let mut start=Instant::now();
    loop {
        let n = reader.read(&mut buf)?; if n==0 { break; }
        file.write_all(&buf[..n])?;
        if let Some(r) = rate { downloaded+=n as u64; let elapsed=start.elapsed(); if elapsed.as_secs_f64()<1.0 && downloaded>=r { std::thread::sleep(Duration::from_secs_f64(1.0)-elapsed); downloaded=0; start=Instant::now(); } else if elapsed.as_secs_f64()>=1.0 { downloaded=0; start=Instant::now(); } }
    }
    Ok(())
}

fn total_size(url: &str) -> Option<f64> {
    let u = if url.starts_with("http") { url.to_string() } else { format!("https://{}", url) };
    let resp = ureq::head(&u).call().ok()?;
    let cl = resp.header("Content-Length")?;
    cl.parse::<f64>().ok().map(|v| v/1024.0/1024.0)
}

fn main() {
    let args = parse_args();
    let mut url = args.url.clone();
    if args.raw { if let Some(c) = github_to_jsdelivr(&url) { println!("Converted GitHub blob -> jsDelivr: {}", c); url=c; } }

    if !url.starts_with("http") { url=format!("https://{}",url); }
    if args.nogui { hide(); }
    if !args.out.exists() { fs::create_dir_all(&args.out).unwrap_or_else(|e| { eprintln!("Failed to create folder {}: {}", args.out.display(), e); exit(1); }); }

    let mut out_path = args.out.clone();
    let file_name=fname(&url);
    out_path.push(&file_name);
    if out_path.exists() { fs::remove_file(&out_path).unwrap_or_else(|e| { eprintln!("Failed to remove existing file {}: {}", out_path.display(), e); exit(1); }); }

    let start=Instant::now();
    let display = fs::canonicalize(&args.out).unwrap_or(args.out.clone()).display().to_string();
    let display = display.strip_prefix(r"\\?\").unwrap_or(&display);

    if !args.nogui {
        if cfg!(target_os="windows") { let _ = std::process::Command::new("cmd").args(["/C","cls"]).status(); } else { print!("\x1B[2J\x1B[1;1H"); }
        println!("Atmospect Homing Installer [Version 1.0.0]");
        println!("(C) Forgotten Private Coalition\n");
        println!("Path: {}", url);
        println!("Output: {}", display);
        println!("Limit: {}\n", lim_str(&args.limit));

        let dl_path=out_path.clone(); let dl_url=url.clone(); let dl_limit=args.limit.clone();
        let handle = std::thread::spawn(move|| { download(&dl_url,&dl_path,&dl_limit).unwrap(); });

        loop {
            let elapsed=start.elapsed();
            let size = fs::metadata(&out_path).map(|m| m.len() as f64/1024.0/1024.0).unwrap_or(0.0);
            let speed=size/elapsed.as_secs_f64().max(0.001);
            let total=total_size(&url);
            let eta_s = total.map_or("-".to_string(), |t| if speed>0.0 { eta((t-size)/speed) } else {"-".to_string()});
            let t_disp=total.unwrap_or(0.0);

            print!("\r\x1B[2K\x1B[?25lDownloaded: {:.3} MB / {:.3} MB | Speed: {:.2} MB/s | ETA: {}", size, t_disp, speed, eta_s);
            io::stdout().flush().unwrap();

            if handle.is_finished() { println!("\n\x1B[?25hDownload finished"); break; }
            std::thread::sleep(Duration::from_millis(100));
        }
    } else {
        if let Err(e)=download(&url,&out_path,&args.limit) { eprintln!("Download failed: {}",e); exit(1); }
    }
}
