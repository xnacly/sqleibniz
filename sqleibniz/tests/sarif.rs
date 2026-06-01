use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct TempSql {
    path: PathBuf,
}

impl TempSql {
    fn new(name: &str, content: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "sqleibniz-{name}-{}-{}.sql",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, content).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSql {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn temp_sql(name: &str, content: &str) -> TempSql {
    TempSql::new(name, content)
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

fn file_arg(file: &TempSql) -> &OsStr {
    file.path().as_os_str()
}

fn file_uri(file: &TempSql) -> String {
    file.path().as_os_str().to_string_lossy().into_owned()
}

#[test]
fn sarif_success_stdout_is_parseable_json() {
    let file = temp_sql("valid", "VACUUM;");

    let output = sqleibniz(&[arg("--sarif"), arg("--ignore-config"), file_arg(&file)]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["version"], "2.1.0");
    assert_eq!(json["runs"][0]["tool"]["driver"]["name"], "sqleibniz");
    assert_eq!(json["runs"][0]["results"].as_array().unwrap().len(), 0);
}

#[test]
fn sarif_diagnostic_uses_rule_message_and_location() {
    let file = temp_sql("invalid", "SELECT");

    let output = sqleibniz(&[arg("--sarif"), arg("--ignore-config"), file_arg(&file)]);

    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let result = &json["runs"][0]["results"][0];
    assert_eq!(result["level"], "error");
    assert!(result["ruleId"].as_str().unwrap().len() > 0);
    assert!(result["message"]["text"].as_str().unwrap().len() > 0);
    assert_eq!(
        result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        file_uri(&file)
    );
    assert!(
        result["locations"][0]["physicalLocation"]["region"]["startLine"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(
        result["locations"][0]["physicalLocation"]["region"]["startColumn"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

#[test]
fn sarif_omits_disabled_rules() {
    let file = temp_sql("disabled", "");

    let output = sqleibniz(&[
        arg("--sarif"),
        arg("--ignore-config"),
        arg("-D"),
        arg("no-content"),
        file_arg(&file),
    ]);

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["runs"][0]["results"].as_array().unwrap().len(), 0);
}

#[test]
fn sarif_conflicts_with_human_and_ast_modes() {
    for flag in ["--silent", "--kiss", "--ast", "--ast-json", "--lsp"] {
        let output = sqleibniz(&[arg("--sarif"), arg(flag)]);

        assert!(!output.status.success(), "{flag} should conflict");
        assert!(output.stdout.is_empty(), "{flag} should not write stdout");
    }
}

#[test]
fn sarif_missing_paths_and_unreadable_files_do_not_write_json() {
    let missing_paths = sqleibniz(&[arg("--sarif")]);
    assert!(!missing_paths.status.success());
    assert!(missing_paths.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing_paths.stderr).contains("no source file"));

    let missing_file = sqleibniz(&[
        arg("--sarif"),
        arg("--ignore-config"),
        arg("does-not-exist.sql"),
    ]);
    assert!(!missing_file.status.success());
    assert!(missing_file.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing_file.stderr).contains("failed to read file"));
}
