/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use super::{is_headless_run, print::MessageWriter};
use core::fmt;
use godot::builtin::{GString, PackedStringArray};

#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error during config parsing: {}", self.message)
    }
}

#[derive(Default)]
pub(crate) struct CliConfig {
    disallow_focus: bool,
    allow_focus: bool,
    disallow_skip: bool,
    allow_skip: bool,
    mute_keyword: bool,
    ignore_keywords: bool,
    mute_filters: bool,
    run_rust_tests: bool,
    run_rust_benchmarks: bool,
    keyword: String,
    filters: Vec<String>,
}

impl CliConfig {
    pub const CMD_USER_RUST_TESTS: &'static str = "--rust-test";
    pub const CMD_USER_RUST_BENCHMARKS: &'static str = "--rust-bench";
    pub const CMD_USER_DISALLOW_FOCUS: &'static str = "--disallow-focus";
    pub const CMD_USER_ALLOW_FOCUS: &'static str = "--allow-focus";
    pub const CMD_USER_DISALLOW_SKIP: &'static str = "--disallow-skip";
    pub const CMD_USER_ALLOW_SKIP: &'static str = "--allow-skip";
    pub const CMD_USER_MUTE_KEYWORD: &'static str = "--mute-keyword";
    pub const CMD_USER_IGNORE_KEYWORDS: &'static str = "--ignore-keywords";
    pub const CMD_USER_MUTE_FILTERS: &'static str = "--mute-filters";
    pub const CMD_USER_KEYWORD: &'static str = "--keyword";
    pub const CMD_USER_FILTERS: &'static str = "--filters";

    pub fn from_os() -> Result<Self, ConfigError> {
        let args = godot::engine::Os::singleton().get_cmdline_user_args();
        let mut args_vec = args.as_slice().iter().collect::<Vec<_>>();

        let run_rust_tests = Self::get_arg(&mut args_vec, Self::CMD_USER_RUST_TESTS);
        let run_rust_benchmarks = Self::get_arg(&mut args_vec, Self::CMD_USER_RUST_BENCHMARKS);

        let allow_focus = Self::get_arg(&mut args_vec, Self::CMD_USER_ALLOW_FOCUS);
        let disallow_focus = Self::get_arg(&mut args_vec, Self::CMD_USER_DISALLOW_FOCUS);

        Self::check_mutually_exclusive_args(
            allow_focus,
            disallow_focus,
            Self::CMD_USER_ALLOW_FOCUS,
            Self::CMD_USER_DISALLOW_FOCUS,
        )?;

        let allow_skip = Self::get_arg(&mut args_vec, Self::CMD_USER_ALLOW_SKIP);
        let disallow_skip = Self::get_arg(&mut args_vec, Self::CMD_USER_DISALLOW_SKIP);

        Self::check_mutually_exclusive_args(
            allow_skip,
            disallow_skip,
            Self::CMD_USER_ALLOW_SKIP,
            Self::CMD_USER_DISALLOW_SKIP,
        )?;

        let mute_keyword = Self::get_arg(&mut args_vec, Self::CMD_USER_MUTE_KEYWORD);
        let ignore_keywords = Self::get_arg(&mut args_vec, Self::CMD_USER_IGNORE_KEYWORDS);

        let keyword_arg = Self::get_arg_with_value(&mut args_vec, Self::CMD_USER_KEYWORD);
        let keyword = if keyword_arg.is_empty() {
            "".to_owned()
        } else {
            keyword_arg[0].to_owned()
        };

        Self::check_mutually_exclusive_args(
            mute_keyword,
            !keyword.is_empty(),
            Self::CMD_USER_MUTE_KEYWORD,
            Self::CMD_USER_KEYWORD,
        )?;

        let mute_filters = Self::get_arg(&mut args_vec, Self::CMD_USER_MUTE_FILTERS);
        let filters = Self::get_arg_with_value(&mut args_vec, Self::CMD_USER_FILTERS);

        Self::check_mutually_exclusive_args(
            mute_filters,
            !filters.is_empty(),
            Self::CMD_USER_MUTE_FILTERS,
            Self::CMD_USER_FILTERS,
        )?;

        let unrecognized_args = args_vec
            .iter()
            .map(|str| str.to_string())
            .collect::<Vec<_>>();
        Self::check_unrecognized_args(&unrecognized_args)?;

        Ok(Self {
            disallow_focus,
            allow_focus,
            disallow_skip,
            allow_skip,
            mute_keyword,
            ignore_keywords,
            mute_filters,
            run_rust_tests,
            run_rust_benchmarks,
            keyword,
            filters,
        })
    }

    fn check_unrecognized_args(unrecognized_args: &Vec<String>) -> Result<(), ConfigError> {
        if unrecognized_args.is_empty() {
            return Ok(());
        }
        Err(ConfigError::new(format!(
            "unrecognized args provided: {:#?}",
            unrecognized_args
        )))
    }

    fn check_mutually_exclusive_args(
        arg_1_val: bool,
        arg_2_val: bool,
        arg_1: &str,
        arg_2: &str,
    ) -> Result<(), ConfigError> {
        match (arg_1_val, arg_2_val) {
            (true, true) => Err(ConfigError::new(format!(
                "command line arguments {} and {} are mutually exclusive",
                arg_1, arg_2
            ))),
            _ => Ok(()),
        }
    }

    fn get_arg(args: &mut Vec<&GString>, get_arg: impl Into<GString>) -> bool {
        let mut gotten = false;
        let get_arg: GString = get_arg.into();
        for (i, arg) in args.iter_mut().enumerate() {
            let cur_arg = arg.clone();
            if cur_arg == get_arg {
                gotten = true;
                args.remove(i);
                break;
            }
        }
        gotten
    }

    fn get_arg_with_value(args: &mut Vec<&GString>, get_arg: &str) -> Vec<String> {
        for (i, arg) in args.iter_mut().enumerate() {
            let cur_arg = arg.clone();
            let arg_str = cur_arg.to_string();
            if arg_str.starts_with(get_arg) {
                let values = arg_str.split('=').collect::<Vec<_>>()[1];
                let values = values
                    .split(',')
                    .map(|str| str.to_owned())
                    .collect::<Vec<String>>();
                args.remove(i);
                return values;
            }
        }
        Vec::new()
    }
}

#[derive(Default)]
pub(crate) struct RunnerConfig {
    disallow_focus: bool,
    disallow_skip: bool,
    run_rust_tests: bool,
    run_rust_benchmarks: bool,
    keyword: String,
    ignore_keywords: bool,
    filters: Vec<String>,
}

impl RunnerConfig {
    pub fn disallow_focus(&self) -> bool {
        self.disallow_focus
    }

    pub fn disallow_skip(&self) -> bool {
        self.disallow_skip
    }

    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    pub fn ignore_keywords(&self) -> bool {
        self.ignore_keywords
    }

    pub fn filters(&self) -> &Vec<String> {
        &self.filters
    }

    pub fn run_rust_tests(&self) -> bool {
        self.run_rust_tests
    }

    pub fn run_rust_benchmarks(&self) -> bool {
        self.run_rust_benchmarks
    }

    pub fn new(
        disallow_focus: bool,
        disallow_skip: bool,
        run_rust_tests: bool,
        run_rust_benchmarks: bool,
        keyword: &GString,
        ignore_keywords: bool,
        filters: &PackedStringArray,
    ) -> Result<Self, ConfigError> {
        let keyword = keyword.to_string();
        let filters = filters
            .as_slice()
            .iter()
            .map(|str| str.to_string())
            .collect::<Vec<_>>();

        let mut instance = Self {
            disallow_focus,
            disallow_skip,
            run_rust_tests,
            run_rust_benchmarks,
            ignore_keywords,
            keyword,
            filters,
        };

        if !is_headless_run() {
            return Ok(instance);
        }

        let cmdline = CliConfig::from_os()?;

        if cmdline.run_rust_tests || cmdline.run_rust_benchmarks {
            instance.run_rust_tests = cmdline.run_rust_tests;
            instance.run_rust_benchmarks = cmdline.run_rust_benchmarks;
        }
        if cmdline.allow_focus {
            instance.disallow_focus = false
        };
        if cmdline.disallow_focus {
            instance.disallow_focus = true
        };
        if cmdline.allow_skip {
            instance.disallow_skip = false
        };
        if cmdline.disallow_skip {
            instance.disallow_skip = true
        };
        if cmdline.mute_filters {
            instance.filters = Vec::new()
        };
        if !cmdline.filters.is_empty() {
            instance.filters = cmdline.filters.clone()
        };
        if cmdline.mute_keyword {
            instance.keyword = String::new()
        };
        if cmdline.ignore_keywords {
            instance.ignore_keywords = true;
        }
        if !cmdline.keyword.is_empty() {
            instance.keyword = cmdline.keyword.clone()
        };

        Ok(instance)
    }

    pub fn print_info(&self) {
        let writer = MessageWriter::new();

        if is_headless_run() {
            writer.println(&format!("{:^80}\n", "Began run in HEADLESS mode"));
        } else {
            writer.println(&format!("{:^80}\n", "Began run in EDITOR mode"));
        }

        let mut additional_message = Vec::new();
        if !self.keyword().is_empty() {
            additional_message.push(format!("using KEYWORD: '{}'", self.keyword()));
        }
        if self.disallow_focus() {
            additional_message.push("disallowing focused".to_owned());
        }
        if self.disallow_skip() {
            additional_message.push("disallowing skipping".to_owned());
        }

        if !additional_message.is_empty() {
            writer.println(&format!("{:^80}\n", additional_message.join(" & ")));
        }

        if !self.filters().is_empty() {
            writer.println(&format!(
                "   Using filters:\n   * {}\n",
                self.filters().join("\n   * ")
            ));
        }
    }
}
