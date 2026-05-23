//! 終端訊息輸出（走 stderr，stdout 留給轉錄結果）。
use std::io::IsTerminal;
use std::sync::OnceLock;

fn color_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::io::stderr().is_terminal())
}

fn paint(code: &str, s: &str) -> String {
    if color_on() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn info(msg: &str) {
    eprintln!("{msg}");
}
pub fn ok(msg: &str) {
    eprintln!("{}", paint("32", &format!("✅ {msg}")));
}
pub fn warn(msg: &str) {
    eprintln!("{}", paint("33", &format!("⚠️  {msg}")));
}
pub fn error(msg: &str) {
    eprintln!("{}", paint("31", &format!("錯誤：{msg}")));
}
pub fn fix(hint: &str) {
    eprintln!("{}", paint("2", &format!("修復：{hint}")));
}
pub fn dim(msg: &str) {
    eprintln!("{}", paint("2", msg));
}
