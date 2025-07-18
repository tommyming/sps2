//! `CMake` build system implementation

use super::{BuildSystem, BuildSystemConfig, BuildSystemContext, TestFailure, TestResults};
use async_trait::async_trait;
use sps2_errors::{BuildError, Error};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// `CMake` build system
pub struct CMakeBuildSystem {
    config: BuildSystemConfig,
}

impl CMakeBuildSystem {
    /// Create a new `CMake` build system instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: BuildSystemConfig {
                supports_out_of_source: true,
                supports_parallel_builds: true,
                supports_incremental_builds: true,
                default_configure_args: vec![
                    "-DCMAKE_BUILD_TYPE=Release".to_string(),
                    "-DCMAKE_COLOR_MAKEFILE=ON".to_string(),
                ],
                default_build_args: vec![],
                env_prefix: Some("CMAKE_".to_string()),
                watch_patterns: vec![
                    "CMakeLists.txt".to_string(),
                    "*.cmake".to_string(),
                    "cmake/*.cmake".to_string(),
                ],
            },
        }
    }

    /// Add macOS-specific RPATH and install name arguments
    fn add_macos_rpath_args(
        args: &mut Vec<String>,
        ctx: &BuildSystemContext,
        user_args: &[String],
    ) {
        // Set install RPATH to where libraries will be installed
        if !user_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_INSTALL_RPATH="))
        {
            args.push(format!(
                "-DCMAKE_INSTALL_RPATH={}/lib",
                ctx.env.get_live_prefix()
            ));
        }

        // Enable macOS RPATH support
        if !user_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_MACOSX_RPATH="))
        {
            args.push("-DCMAKE_MACOSX_RPATH=ON".to_string());
        }

        // Don't use install RPATH during build (use build RPATH)
        if !user_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_BUILD_WITH_INSTALL_RPATH="))
        {
            args.push("-DCMAKE_BUILD_WITH_INSTALL_RPATH=OFF".to_string());
        }

        // Add RPATH entries for all linked library directories
        if !user_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_INSTALL_RPATH_USE_LINK_PATH="))
        {
            args.push("-DCMAKE_INSTALL_RPATH_USE_LINK_PATH=ON".to_string());
        }

        // If with_defaults() was called, set CMAKE_INSTALL_NAME_DIR to prevent self-referencing install names
        if ctx.env.with_defaults_called
            && !user_args
                .iter()
                .any(|arg| arg.starts_with("-DCMAKE_INSTALL_NAME_DIR="))
        {
            args.push("-DCMAKE_INSTALL_NAME_DIR=/opt/pm/live/lib".to_string());
        }
    }

    /// Get `CMake` configuration arguments
    fn get_cmake_args(&self, ctx: &BuildSystemContext, user_args: &[String]) -> Vec<String> {
        let mut args = vec![];

        // Always specify source directory
        args.push(ctx.source_dir.display().to_string());

        // Add install prefix - use LIVE_PREFIX for runtime installation location
        if !user_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_INSTALL_PREFIX="))
        {
            args.push(format!(
                "-DCMAKE_INSTALL_PREFIX={}",
                ctx.env.get_live_prefix()
            ));
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

        // Set RPATH for macOS to ensure binaries can find their libraries
        if cfg!(target_os = "macos") {
            Self::add_macos_rpath_args(&mut args, ctx, user_args);
        }

        // macOS ARM only - no cross-compilation support

        // Add CMAKE_PREFIX_PATH from build dependencies
        if let Some(pkg_config_path) = ctx.get_all_env_vars().get("PKG_CONFIG_PATH") {
            let prefix_paths: Vec<String> = pkg_config_path
                .split(':')
                .filter_map(|p| {
                    Path::new(p)
                        .parent()
                        .and_then(|p| p.parent())
                        .map(|p| p.display().to_string())
                })
                .collect();

            if !prefix_paths.is_empty() {
                args.push(format!("-DCMAKE_PREFIX_PATH={}", prefix_paths.join(";")));
            }
        }

        // Add find_package hints
        if !user_args
            .iter()
            .any(|arg| arg.starts_with("-DCMAKE_FIND_PACKAGE_PREFER_CONFIG="))
        {
            args.push("-DCMAKE_FIND_PACKAGE_PREFER_CONFIG=ON".to_string());
        }

        // Add user arguments
        args.extend(user_args.iter().cloned());

        args
    }
}

impl Default for CMakeBuildSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BuildSystem for CMakeBuildSystem {
    async fn detect(&self, source_dir: &Path) -> Result<bool, Error> {
        Ok(source_dir.join("CMakeLists.txt").exists())
    }

    fn get_config_options(&self) -> BuildSystemConfig {
        self.config.clone()
    }

    async fn configure(&self, ctx: &BuildSystemContext, args: &[String]) -> Result<(), Error> {
        // Create build directory for out-of-source build
        if ctx.source_dir != ctx.build_dir {
            fs::create_dir_all(&ctx.build_dir).await?;
        }

        // Build CMake command
        let cmake_args = self.get_cmake_args(ctx, args);
        let arg_refs: Vec<&str> = cmake_args.iter().map(String::as_str).collect();

        // Run cmake
        let result = ctx
            .execute("cmake", &arg_refs, Some(&ctx.build_dir))
            .await?;

        if !result.success {
            return Err(BuildError::ConfigureFailed {
                message: format!("cmake configuration failed: {}", result.stderr),
            }
            .into());
        }

        Ok(())
    }

    async fn build(&self, ctx: &BuildSystemContext, args: &[String]) -> Result<(), Error> {
        let mut cmake_args = vec!["--build", "."];

        // Add parallel jobs
        let jobs_str;
        if ctx.jobs > 1 {
            jobs_str = ctx.jobs.to_string();
            cmake_args.push("--parallel");
            cmake_args.push(&jobs_str);
        }

        // Add user arguments
        if !args.is_empty() {
            cmake_args.push("--");
            cmake_args.extend(args.iter().map(String::as_str));
        }

        // Run cmake build
        let result = ctx
            .execute("cmake", &cmake_args, Some(&ctx.build_dir))
            .await?;

        if !result.success {
            return Err(BuildError::CompilationFailed {
                message: format!("cmake build failed: {}", result.stderr),
            }
            .into());
        }

        Ok(())
    }

    async fn test(&self, ctx: &BuildSystemContext) -> Result<TestResults, Error> {
        let start = std::time::Instant::now();

        // Run ctest
        let result = ctx
            .execute(
                "ctest",
                &["--output-on-failure", "--parallel", &ctx.jobs.to_string()],
                Some(&ctx.build_dir),
            )
            .await?;

        let duration = start.elapsed().as_secs_f64();
        let output = format!("{}\n{}", result.stdout, result.stderr);

        // Parse CTest output
        let mut total = 0;
        let mut passed = 0;
        let mut failed = 0;
        let mut failures = vec![];

        for line in output.lines() {
            // Look for test summary line: "X% tests passed, Y tests failed out of Z"
            if line.contains("% tests passed") {
                if let Some(summary) = parse_ctest_summary(line) {
                    total = summary.0;
                    passed = summary.1;
                    failed = summary.2;
                }
            }
            // Capture test failures
            else if line.contains("***Failed") || line.contains("***Timeout") {
                if let Some(test_name) = line.split_whitespace().nth(1) {
                    failures.push(TestFailure {
                        name: test_name.to_string(),
                        message: line.to_string(),
                        details: None,
                    });
                }
            }
        }

        // If no summary found but command failed, assume all tests failed
        if total == 0 && !result.success {
            total = 1;
            failed = 1;
        }

        Ok(TestResults {
            total,
            passed,
            failed,
            skipped: 0,
            duration,
            output,
            failures,
        })
    }

    async fn install(&self, ctx: &BuildSystemContext) -> Result<(), Error> {
        // When using DESTDIR, we need to adjust the install behavior
        // DESTDIR is prepended to the install prefix, so if CMAKE_INSTALL_PREFIX is /opt/pm/live
        // and DESTDIR is /path/to/stage, files go to /path/to/stage/opt/pm/live
        // To get files in /path/to/stage directly, we need to strip the prefix during packaging

        // Set DESTDIR in the environment for this context
        let staging_dir = ctx.env.staging_dir().display().to_string();
        if let Ok(mut extra) = ctx.extra_env.write() {
            extra.insert("DESTDIR".to_string(), staging_dir.clone());
        }

        // Also set it directly in the build environment
        // Set DESTDIR in the environment - env_vars is not a RwLock, it's a HashMap
        // We need to use a different approach
        std::env::set_var("DESTDIR", &staging_dir);

        let result = ctx
            .execute("cmake", &["--install", "."], Some(&ctx.build_dir))
            .await;

        match result {
            Ok(res) if res.success => {
                // No need to adjust staged files since we're using BUILD_PREFIX now
                Ok(())
            }
            _ => {
                // Fallback to make install for older CMake versions or if cmake --install fails
                // Ensure DESTDIR is set in environment
                // Set DESTDIR in the environment
                std::env::set_var("DESTDIR", &staging_dir);

                let make_result = ctx
                    .execute("make", &["install"], Some(&ctx.build_dir))
                    .await?;

                if !make_result.success {
                    return Err(BuildError::InstallFailed {
                        message: format!("cmake install failed: {}", make_result.stderr),
                    }
                    .into());
                }

                // No need to adjust staged files since we're using BUILD_PREFIX now
                Ok(())
            }
        }
    }

    fn get_env_vars(&self, ctx: &BuildSystemContext) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        // Set DESTDIR for install phase
        vars.insert(
            "DESTDIR".to_string(),
            ctx.env.staging_dir().display().to_string(),
        );

        // CMake-specific environment variables
        if let Some(cache_config) = &ctx.cache_config {
            if cache_config.use_compiler_cache {
                match cache_config.compiler_cache_type {
                    super::core::CompilerCacheType::CCache => {
                        vars.insert(
                            "CMAKE_CXX_COMPILER_LAUNCHER".to_string(),
                            "ccache".to_string(),
                        );
                        vars.insert(
                            "CMAKE_C_COMPILER_LAUNCHER".to_string(),
                            "ccache".to_string(),
                        );
                    }
                    super::core::CompilerCacheType::SCCache => {
                        vars.insert(
                            "CMAKE_CXX_COMPILER_LAUNCHER".to_string(),
                            "sccache".to_string(),
                        );
                        vars.insert(
                            "CMAKE_C_COMPILER_LAUNCHER".to_string(),
                            "sccache".to_string(),
                        );
                    }
                    super::core::CompilerCacheType::DistCC => {}
                }
            }
        }

        vars
    }

    fn name(&self) -> &'static str {
        "cmake"
    }

    fn prefers_out_of_source_build(&self) -> bool {
        true
    }
}

/// Parse `CTest` summary line
fn parse_ctest_summary(line: &str) -> Option<(usize, usize, usize)> {
    // Parse "X% tests passed, Y tests failed out of Z"
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Find "failed" and "of" positions
    let failed_pos = parts.iter().position(|&s| s == "failed")?;
    let out_of_pos = parts.iter().position(|&s| s == "of")?;

    // Look for the number before "tests failed"
    // The pattern is "N tests failed", so we need to go back 2 positions from "failed"
    let failed = if failed_pos >= 2 && parts.get(failed_pos - 1) == Some(&"tests") {
        parts.get(failed_pos - 2)?.parse().ok()?
    } else {
        parts.get(failed_pos - 1)?.parse().ok()?
    };

    let total = parts.get(out_of_pos + 1)?.parse().ok()?;
    let passed = total - failed;

    Some((total, passed, failed))
}
