//! Meson build system implementation

use super::{BuildSystem, BuildSystemConfig, BuildSystemContext, TestFailure, TestResults};
use async_trait::async_trait;
use sps2_errors::{BuildError, Error};
use std::collections::HashMap;
use std::path::Path;

/// Meson build system
pub struct MesonBuildSystem {
    config: BuildSystemConfig,
}

impl MesonBuildSystem {
    /// Create a new Meson build system instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: BuildSystemConfig {
                supports_out_of_source: true,
                supports_parallel_builds: true,
                supports_incremental_builds: true,
                default_configure_args: vec![
                    "--buildtype=release".to_string(),
                    "--optimization=2".to_string(),
                    "--strip".to_string(),
                ],
                default_build_args: vec![],
                env_prefix: Some("MESON_".to_string()),
                watch_patterns: vec![
                    "meson.build".to_string(),
                    "meson_options.txt".to_string(),
                    "*/meson.build".to_string(),
                ],
            },
        }
    }

    /// Check if wrap handling should be disabled
    fn should_disable_wrap(ctx: &BuildSystemContext) -> bool {
        // Always disable wrap downloads to ensure reproducible builds
        !ctx.network_allowed
    }

    /// Get Meson setup arguments
    fn get_setup_args(&self, ctx: &BuildSystemContext, user_args: &[String]) -> Vec<String> {
        let mut args = vec!["setup".to_string()];

        // Build directory
        args.push(ctx.build_dir.display().to_string());

        // Source directory (if different from current)
        if ctx.source_dir != std::env::current_dir().unwrap_or_default() {
            args.push(ctx.source_dir.display().to_string());
        }

        // Add prefix - use LIVE_PREFIX for runtime installation location
        if !user_args.iter().any(|arg| arg.starts_with("--prefix=")) {
            args.push(format!("--prefix={}", ctx.env.get_live_prefix()));
        }

        // Add default arguments
        for default_arg in &self.config.default_configure_args {
            if !user_args
                .iter()
                .any(|arg| arg.starts_with(default_arg.split('=').next().unwrap_or("")))
            {
                args.push(default_arg.clone());
            }
        }

        // Handle wrap mode
        if Self::should_disable_wrap(ctx)
            && !user_args.iter().any(|arg| arg.starts_with("--wrap-mode="))
        {
            args.push("--wrap-mode=nodownload".to_string());
        }

        // macOS ARM only - no cross-compilation support

        // Add PKG_CONFIG_PATH
        if let Some(pkg_config_path) = ctx.get_all_env_vars().get("PKG_CONFIG_PATH") {
            args.push(format!("--pkg-config-path={pkg_config_path}"));
        }

        // Add user arguments
        args.extend(user_args.iter().cloned());

        args
    }

    /// Parse Meson test output
    fn parse_test_output(output: &str) -> (usize, usize, usize, Vec<TestFailure>) {
        let mut total = 0;
        let mut passed = 0;
        let mut failed = 0;
        let mut failures = vec![];

        for line in output.lines() {
            // Meson test output format: "1/4 test_name        OK              0.12s"
            if let Some((test_num, test_name, status)) = parse_meson_test_line(line) {
                total = total.max(test_num);

                match status {
                    "OK" | "EXPECTEDFAIL" => passed += 1,
                    "FAIL" | "TIMEOUT" => {
                        failed += 1;
                        failures.push(TestFailure {
                            name: test_name.to_string(),
                            message: format!("Test {test_name} failed with status: {status}"),
                            details: None,
                        });
                    }
                    _ => {} // Don't count as passed or failed (includes SKIP)
                }
            }
        }

        (total, passed, failed, failures)
    }
}

impl Default for MesonBuildSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BuildSystem for MesonBuildSystem {
    async fn detect(&self, source_dir: &Path) -> Result<bool, Error> {
        Ok(source_dir.join("meson.build").exists())
    }

    fn get_config_options(&self) -> BuildSystemConfig {
        self.config.clone()
    }

    async fn configure(&self, ctx: &BuildSystemContext, args: &[String]) -> Result<(), Error> {
        // Get setup arguments
        let setup_args = self.get_setup_args(ctx, args);
        let arg_refs: Vec<&str> = setup_args.iter().map(String::as_str).collect();

        // Run meson setup
        let result = ctx
            .execute("meson", &arg_refs, Some(&ctx.source_dir))
            .await?;

        if !result.success {
            return Err(BuildError::ConfigureFailed {
                message: format!("meson setup failed: {}", result.stderr),
            }
            .into());
        }

        Ok(())
    }

    async fn build(&self, ctx: &BuildSystemContext, args: &[String]) -> Result<(), Error> {
        let mut compile_args = vec!["compile"];

        // Add parallel jobs
        let jobs_str;
        if ctx.jobs > 1 {
            jobs_str = ctx.jobs.to_string();
            compile_args.push("-j");
            compile_args.push(&jobs_str);
        }

        // Add build directory
        let build_dir_str = ctx.build_dir.display().to_string();
        compile_args.push("-C");
        compile_args.push(&build_dir_str);

        // Add user arguments
        let user_arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        compile_args.extend(user_arg_refs);

        // Run meson compile
        let result = ctx
            .execute("meson", &compile_args, Some(&ctx.source_dir))
            .await?;

        if !result.success {
            return Err(BuildError::CompilationFailed {
                message: format!("meson compile failed: {}", result.stderr),
            }
            .into());
        }

        Ok(())
    }

    async fn test(&self, ctx: &BuildSystemContext) -> Result<TestResults, Error> {
        let start = std::time::Instant::now();

        // Run meson test
        let build_dir_str = ctx.build_dir.display().to_string();
        let jobs_str = ctx.jobs.to_string();
        let result = ctx
            .execute(
                "meson",
                &[
                    "test",
                    "-C",
                    &build_dir_str,
                    "--print-errorlogs",
                    "--num-processes",
                    &jobs_str,
                ],
                Some(&ctx.source_dir),
            )
            .await?;

        let duration = start.elapsed().as_secs_f64();
        let output = format!("{}\n{}", result.stdout, result.stderr);

        // Parse test results
        let (total, passed, failed, failures) = Self::parse_test_output(&output);
        let skipped = total.saturating_sub(passed + failed);

        Ok(TestResults {
            total,
            passed,
            failed,
            skipped,
            duration,
            output,
            failures,
        })
    }

    async fn install(&self, ctx: &BuildSystemContext) -> Result<(), Error> {
        // Set DESTDIR for staged installation
        let staging_dir = ctx.env.staging_dir();
        if let Ok(mut extra_env) = ctx.extra_env.write() {
            extra_env.insert("DESTDIR".to_string(), staging_dir.display().to_string());
        }

        // Run meson install
        let build_dir_str = ctx.build_dir.display().to_string();
        let result = ctx
            .execute(
                "meson",
                &["install", "-C", &build_dir_str],
                Some(&ctx.source_dir),
            )
            .await?;

        if !result.success {
            return Err(BuildError::InstallFailed {
                message: format!("meson install failed: {}", result.stderr),
            }
            .into());
        }

        // No need to adjust staged files since we're using BUILD_PREFIX now
        Ok(())
    }

    fn get_env_vars(&self, _ctx: &BuildSystemContext) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        // Meson uses DESTDIR environment variable for staged installs
        // This is set in the install() method rather than globally

        // Meson-specific environment variables
        vars.insert("MESON_FORCE_COLOR".to_string(), "1".to_string());

        vars
    }

    fn name(&self) -> &'static str {
        "meson"
    }

    fn prefers_out_of_source_build(&self) -> bool {
        // Meson requires out-of-source builds
        true
    }

    fn build_directory_name(&self) -> &'static str {
        "builddir"
    }
}

/// Parse a Meson test output line
fn parse_meson_test_line(line: &str) -> Option<(usize, &str, &str)> {
    // Format: "1/4 test_name        OK              0.12s"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    // Parse test number
    let test_num_str = parts[0];
    if let Some(slash_pos) = test_num_str.find('/') {
        if let Ok(num) = test_num_str[..slash_pos].parse() {
            let test_name = parts[1];
            let status = parts[2];
            return Some((num, test_name, status));
        }
    }

    None
}
