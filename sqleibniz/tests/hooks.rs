use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(name: &str, ext: &str, content: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "sqleibniz-{name}-{}-{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            ext
        ));
        fs::write(&path, content).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn temp_file(name: &str, ext: &str, content: &str) -> TempFile {
    TempFile::new(name, ext, content)
}

fn sqleibniz(args: &[&OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sqleibniz"))
        .args(args)
        .output()
        .unwrap()
}

fn arg(value: &str) -> &OsStr {
    OsStr::new(value)
}

fn file_arg(file: &TempFile) -> &OsStr {
    file.path().as_os_str()
}

fn lowercase_hook_config() -> TempFile {
    temp_file(
        "hooks",
        "lua",
        r#"
leibniz = {
    hooks = {
        {
            name = "idents should be lowercase",
            match = { node = "Token", kind = "Ident" },
            hook = function(node)
                if string.match(node.content, "%u") and not sqleibniz.is_type_name(node) then
                    sqleibniz.diagnostic(node, "ident should be lowercase")
                end
            end
        }
    }
}
"#,
    )
}

fn slow_hook_config(max_hook_runtime: Option<&str>) -> TempFile {
    let max_hook_runtime = max_hook_runtime
        .map(|value| format!("max_hook_runtime = {value},"))
        .unwrap_or_default();
    temp_file(
        "slow-hook",
        "lua",
        &format!(
            r#"
leibniz = {{
    {max_hook_runtime}
    hooks = {{
        {{
            name = "slow hook",
            match = {{ node = "Token", kind = "Ident" }},
            hook = function(_)
                while true do end
            end
        }}
    }}
}}
"#,
        ),
    )
}

#[test]
fn hook_error_reports_diagnostic() {
    let config = lowercase_hook_config();
    let sql = temp_file("uppercase-ident", "sql", "VACUUM UpperName;");

    let output = sqleibniz(&[
        arg("--kiss"),
        arg("--config"),
        file_arg(&config),
        file_arg(&sql),
    ]);

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sqleibniz/hook: idents should be lowercase"));
    assert!(stdout.contains("ident should be lowercase"));
}

#[test]
fn hook_diagnostics_can_be_disabled() {
    let config = lowercase_hook_config();
    let sql = temp_file("uppercase-ident-disabled", "sql", "VACUUM UpperName;");

    let output = sqleibniz(&[
        arg("--kiss"),
        arg("--config"),
        file_arg(&config),
        arg("-D"),
        arg("sqleibniz/hook"),
        file_arg(&sql),
    ]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn hooks_skip_expected_statement_tokens() {
    let config = temp_file(
        "hooks-content",
        "lua",
        r#"
leibniz = {
    hooks = {
        {
            name = "idents should be lowercase",
            match = { node = "Token", kind = "Ident" },
            hook = function(node)
                if string.match(node.content, "%u") then
                    sqleibniz.diagnostic(node, node.content)
                end
            end
        }
    }
}
"#,
    );
    let sql = temp_file(
        "uppercase-ident-expected",
        "sql",
        "-- @sqleibniz::expect\nVACUUM ExpectedName;\nVACUUM ReportedName;",
    );

    let output = sqleibniz(&[
        arg("--kiss"),
        arg("--config"),
        file_arg(&config),
        file_arg(&sql),
    ]);

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("ExpectedName"));
    assert!(stdout.contains("ReportedName"));
}

#[test]
fn hooks_time_out_after_the_default_runtime() {
    let config = slow_hook_config(None);
    let sql = temp_file("slow-hook-default", "sql", "VACUUM name;");

    let output = sqleibniz(&[
        arg("--kiss"),
        arg("--config"),
        file_arg(&config),
        file_arg(&sql),
    ]);

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("slow hook"));
    assert!(stdout.contains("maximum runtime of 10ms"));
}

#[test]
fn config_sets_the_hook_runtime() {
    let config = slow_hook_config(Some("1"));
    let sql = temp_file("slow-hook-config", "sql", "VACUUM name;");

    let output = sqleibniz(&[
        arg("--kiss"),
        arg("--config"),
        file_arg(&config),
        file_arg(&sql),
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("maximum runtime of 1ms"));
}

#[test]
fn cli_hook_runtime_overrides_config() {
    let config = slow_hook_config(Some("1000"));
    let sql = temp_file("slow-hook-cli", "sql", "VACUUM name;");

    let output = sqleibniz(&[
        arg("--kiss"),
        arg("--config"),
        file_arg(&config),
        arg("--max-hook-runtime=0ms"),
        file_arg(&sql),
    ]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("maximum runtime of 0ms"));
}
