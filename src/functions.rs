use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use is_executable::IsExecutable;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use regex::Regex;
use rusty_v8::{
    self as v8, FunctionCallback, FunctionTemplate, HandleScope, Local, MapFnTo, Object,
};
use std::{env, path::Path, process::Command};

use crate::{
    config::CONFIG,
    io::flush,
    path::{expand, expand_path},
};

lazy_static! {
    pub static ref RUNNING: Mutex<bool> = Mutex::new(true);
    pub static ref HISTORY: Mutex<Vec<String>> = {
        let history_file_path = expand_path(&(*CONFIG.lock().history_file.clone()));
        let history = if history_file_path.exists() {
            let history_file = std::fs::read_to_string(history_file_path).unwrap();
            history_file
                .split("\n")
                .map(|entry| String::from(entry))
                .collect::<Vec<String>>()
        } else {
            vec![]
        };

        Mutex::new(history)
    };
    pub static ref HISTORY_POINTER: Mutex<usize> = Mutex::new(0);
    pub static ref EXECUTABLES: Mutex<Vec<String>> = {
        let mut paths = env::var_os("PATH")
            .unwrap()
            .to_string_lossy()
            .split(":")
            .map(|path| path.to_string())
            .collect::<Vec<String>>();
        paths.sort();
        paths.dedup();

        let mut executables = paths
            .iter()
            .map(|path| {
                let children = match std::fs::read_dir(path) {
                    Ok(entry) => entry,
                    Err(_error) => return Vec::new(),
                };

                let executables = children
                    .filter_map(|child| {
                        let path = child.as_ref().unwrap().path();
                        if !path.is_executable() {
                            return None;
                        }
                        Some(path.file_name().unwrap().to_str().unwrap().to_string())
                    })
                    .collect::<Vec<String>>();

                executables
            })
            .flatten()
            .collect::<Vec<String>>();

        executables.sort();
        executables.dedup();
        Mutex::new(executables)
    };
}

pub fn create_functions(scope: &mut HandleScope, global: Local<Object>) {
    create_js_function(
        scope,
        global,
        "$exit",
        |_scope: &mut v8::HandleScope,
         _args: v8::FunctionCallbackArguments,
         _rv: v8::ReturnValue| {
            *RUNNING.lock() = false;
        },
    );

    create_js_function(
        scope,
        global,
        "$find",
        |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let pattern = args.get(0);

            if !pattern.is_string() {
                // TODO: Throw error
                return;
            }

            // args.this().get_property_names(scope);

            let pattern = pattern.to_rust_string_lossy(scope);
            let executables = EXECUTABLES.lock();
            let matcher = SkimMatcherV2::default();

            let results = executables
                .iter()
                .filter_map(|name| {
                    let result = matcher.fuzzy_indices(name, &pattern);
                    if let Some((score, indices)) = result {
                        return Some((name, score, indices));
                    }
                    return None;
                })
                .collect::<Vec<(&String, i64, Vec<usize>)>>();

            for (name, _, _) in results {
                print!("{name}\n\r")
            }
        },
    );

    create_js_function(
        scope,
        global,
        "$setEnv",
        |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let v8_key = args.get(0);
            if !v8_key.is_string() {
                // TODO: Throw error
                return;
            }
            let v8_value = args.get(1);
            if !v8_value.is_string() {
                // TODO: Throw error
                return;
            }
            let key = v8_key.to_rust_string_lossy(scope);
            let value = v8_value.to_rust_string_lossy(scope);
            env::set_var(key, value)
        },
    );

    create_js_function(
        scope,
        global,
        "$getEnv",
        |scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let v8_key = args.get(0);
            if !v8_key.is_string() {
                // TODO: Throw error
                return;
            }
            let key = v8_key.to_rust_string_lossy(scope);
            let value = match env::var(key) {
                Ok(value) => value,
                Err(_error) => {
                    // TODO: Throw error
                    return;
                }
            };

            rv.set(v8::String::new(scope, &value).unwrap().into())
        },
    );

    create_js_function(
        scope,
        global,
        "$history",
        |scope: &mut v8::HandleScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            let return_array = v8::Array::new(scope, 0);
            HISTORY
                .lock()
                .iter()
                .enumerate()
                .for_each(|(index, entry)| {
                    let value = v8::String::new(scope, &entry).unwrap().into();
                    return_array.set_index(scope, index as u32, value);
                });

            rv.set(return_array.into())
        },
    );

    create_js_function(
        scope,
        global,
        "$source",
        |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let file_argument = args.get(0);
            if !file_argument.is_string() {
                // TODO: Throw error
                return;
            }
            let file_path = file_argument.to_rust_string_lossy(scope);
            let file_path_buf = expand_path(&file_path);
            let file_exists = file_path_buf.exists();
            if !file_exists {
                eprintln!("File {file_path} does not exist or missing permissions")
            }
            let source_code = std::fs::read_to_string(file_path_buf).unwrap();

            let code = v8::String::new(scope, &source_code).unwrap();
            let script = v8::Script::compile(scope, code, None).unwrap();
            let _result = script.run(scope).unwrap();
        },
    );

    create_js_function(
        scope,
        global,
        "$run",
        |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let mut arguments = Vec::new();
            let re = Regex::new(r#"("[^"]*")|\S+"#).unwrap();
            for i in 0..args.length() {
                let arg = args.get(i);
                if !arg.is_string() {
                    // TODO: Do something
                    continue;
                }
                arguments.push({
                    re.find_iter(&arg.to_rust_string_lossy(scope))
                        .map(|mat| mat.as_str().to_string())
                        .collect::<Vec<String>>()
                });
            }
            let mut arguments = arguments.iter().flatten().map(|argument| {
                let mut argument = argument.to_string();

                let contains_tilde = argument.starts_with("~");
                let contains_tilde_in_string =
                    argument.starts_with("\"~") && argument.ends_with("\"");

                if contains_tilde || contains_tilde_in_string {
                    argument = expand(&argument)
                }
                argument
            });

            let name = arguments.next().unwrap();
            let mut command = Command::new(name);
            command.args(arguments);
            // print!("\n\r");
            disable_raw_mode().unwrap();
            let mut child = command.spawn().unwrap();

            child.wait().unwrap();
            enable_raw_mode().unwrap();

            flush()
        },
    );

    create_js_function(
        scope,
        global,
        "$drop",
        |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            // TODO: Fix to work with variables, functions and other things. Currently only works with properties directly put onto globalThis.

            let global = scope.get_current_context().global(scope);

            for i in 0..args.length() {
                let argument = args.get(i);
                if !argument.is_string() {
                    continue;
                }
                let name = argument.to_rust_string_lossy(scope);

                let name = v8::String::new(scope, &name).unwrap().into();

                global.delete(scope, name);

                // let value = global.get(scope, argument).unwrap();
                // println!("{}", value.to_rust_string_lossy(scope));

                // global.delete(scope, argument);

                // Drop value here
            }
        },
    );

    // TODO: Implement console.log and console.error;

    // TODO: Convert to `native code` functions instead of javascript
    let template = include_str!("./scripts/command_template.js")
        .replace("\n", "")
        .replace(" ", "");

    let executables = EXECUTABLES.lock();

    let code = v8::String::new(
        scope,
        &executables
            .iter()
            .map(|name| template.replace("{name}", &name))
            .collect::<Vec<String>>()
            .join(""),
    )
    .unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let _result = script.run(scope).unwrap();

    let template = include_str!("./scripts/environment_template.js")
        .replace("\n", "");
        // .replace(" ", "");

    let code = v8::String::new(
        scope,
        &env::vars()
            .map(|(key, _value)| template.replace("{name}", &key))
            .collect::<Vec<String>>()
            .join(""),
    )
    .unwrap();

    let script = v8::Script::compile(scope, code, None).unwrap();
    let _result = script.run(scope).unwrap();

    // env::vars().for_each(|(key, value)| {
    //     let key = v8::String::new(scope, &format!("${key}")).unwrap();

    //     let value = v8::String::new(scope, &value).unwrap();
    //     global.set(scope, key.into(), value.into());
    // });

    /*
     * `cd` is a shell builtin and should be defined after all other executables
     */

    create_js_function(
        scope,
        global,
        "cd",
        |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {
            let path = args.get(0);
            if !path.is_string() && !path.is_undefined() {
                // TODO: Throw error
                return;
            }

            let path_string = path.to_rust_string_lossy(scope);
            let path = if path.is_undefined() {
                expand("~")
            } else {
                expand(&path_string)
            }
            .to_string();

            let root = Path::new(&path);
            if let Err(e) = env::set_current_dir(&root) {
                eprintln!("{}", e);
            }
        },
    );
}

pub fn create_js_function(
    scope: &mut HandleScope,
    global: Local<Object>,
    name: &str,
    callback: impl MapFnTo<FunctionCallback>,
) {
    let function = FunctionTemplate::new(scope, callback)
        .get_function(scope)
        .unwrap();
    let name = v8::String::new(scope, name).unwrap();
    global.set(scope, name.into(), function.into());
}
