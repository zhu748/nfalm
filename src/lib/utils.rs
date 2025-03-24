use figlet_rs::FIGfont;
use std::sync::LazyLock;

pub static BANNER: LazyLock<String> = LazyLock::new(|| {
    let standard_font = FIGfont::standard().unwrap();
    let figure = standard_font.convert("Clewdr");
    let banner = figure.unwrap().to_string();
    format!(
        "{}\nv{} by {}\n",
        banner,
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    )
});

pub const MODELS: [&str; 10] = [
    "claude-3-7-sonnet-20250219",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-sonnet-20240229",
    "claude-3-haiku-20240307",
    "claude-2.1",
    "claude-2.0",
    "claude-1.3",
    "claude-instant-1.2",
    "claude-instant-1.1",
];

pub const ENDPOINT: &str = "https://api.claude.ai";

pub fn header_ref(ref_path: &str) -> String {
    if ref_path.is_empty() {
        format!("{}/", ENDPOINT)
    } else {
        format!("{}/chat/{}", ENDPOINT, ref_path)
    }
}
