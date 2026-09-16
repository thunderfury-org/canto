use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/vite.config.js");

    let dist_dir = Path::new("web/dist");
    let index_html = dist_dir.join("index.html");

    if !index_html.exists() {
        // Try building with npm if available
        let built = Command::new("npm")
            .args(["run", "build"])
            .current_dir("web")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !built || !index_html.exists() {
            // Fallback stub for environments without node/npm (e.g. cross-compilation)
            fs::create_dir_all(dist_dir).expect("failed to create web/dist directory");
            fs::write(
                &index_html,
                r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>canto studio</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #090d16; color: #94a3b8; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
    .box { border: 1px solid #1e293b; background: #0f172a; padding: 28px; border-radius: 12px; max-width: 480px; text-align: center; box-shadow: 0 10px 25px -5px rgba(0,0,0,0.5); }
    h1 { color: #38bdf8; font-size: 20px; margin-top: 0; }
    p { font-size: 14px; line-height: 1.6; color: #cbd5e1; }
    code { background: #1e293b; padding: 3px 8px; border-radius: 6px; color: #38bdf8; font-family: monospace; font-size: 13px; }
  </style>
</head>
<body>
  <div class="box">
    <h1>canto Web Studio</h1>
    <p>前端静态资源尚未构建。在开发或发布环境中，请执行：</p>
    <p><code>cd web && npm install && npm run build</code></p>
  </div>
</body>
</html>"##,
            )
            .expect("failed to write fallback index.html");

            let favicon = dist_dir.join("favicon.svg");
            if !favicon.exists() {
                let _ = fs::write(
                    &favicon,
                    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="#0284c7"/></svg>"##,
                );
            }

            let icons = dist_dir.join("icons.svg");
            if !icons.exists() {
                let _ = fs::write(
                    &icons,
                    r##"<svg xmlns="http://www.w3.org/2000/svg"></svg>"##,
                );
            }
        }
    }
}
