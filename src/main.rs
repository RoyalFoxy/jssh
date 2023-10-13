use crossterm::{
    cursor::{MoveLeft, RestorePosition, SavePosition},
    event::{
        self, KeyCode, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use jssh::{
    config::CONFIG,
    functions::{create_functions, HISTORY, HISTORY_POINTER, RUNNING},
    highlight::Highlighter,
    io::{flush, NEWLINE_RETURN},
    path::expand_path,
};
use rusty_v8::{self as v8, V8};
use std::{io::stdout, panic};
use v8::HandleScope;

static PROMPT: &str = ">";

fn de_init() {
    unsafe { V8::dispose() };
    V8::shutdown_platform();
    disable_raw_mode().unwrap();
    execute!(stdout(), PopKeyboardEnhancementFlags).unwrap();
    std::fs::write(
        expand_path(&(*CONFIG.lock().history_file.clone())),
        (*HISTORY.lock()).join("\n"),
    )
    .unwrap();
}

fn main() -> anyhow::Result<()> {
    panic::set_hook(Box::new(|info| {
        de_init();
        println!("{info}");
    }));

    let start_up_file = expand_path(&*CONFIG.lock().start_up_file.clone());
    if !start_up_file.exists() {
        std::fs::write(start_up_file, include_str!("./scripts/start_up_file.js"))?;
    }

    enable_raw_mode()?;
    execute!(
        stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .unwrap();

    {
        let platform = v8::new_default_platform(0, false).make_shared();

        V8::initialize_platform(platform);
        V8::initialize();

        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let handle_scope = &mut v8::HandleScope::new(isolate);

        let context = v8::Context::new(handle_scope);

        let context_scope = &mut v8::ContextScope::new(handle_scope, context);
        let scope = &mut v8::HandleScope::new(context_scope);

        let global = context.global(scope);

        create_functions(scope, global);

        let source_code = format!(r#"$source("{}")"#, &*CONFIG.lock().start_up_file);

        let v8_source_code = v8::String::new(scope, &source_code).unwrap();
        let v8_script = v8::Script::compile(scope, v8_source_code, None).unwrap();
        v8_script.run(scope).unwrap();

        let highlighter = &mut Highlighter::new();

        while *RUNNING.lock() {
            let code = loop_callback(scope, highlighter)?;
            match code {
                LoopCodes::Ok => (),
                LoopCodes::Exit => break,
                LoopCodes::Cancelled => (),
                LoopCodes::CompilationFailed => print!("Compilation Error{NEWLINE_RETURN}"),
                LoopCodes::RuntimeFailed => print!("Runtime Error{NEWLINE_RETURN}"),
            };
        }
    }

    de_init();
    Ok(())
}

pub enum LoopCodes {
    Ok = 0,
    Exit = 1,
    Cancelled = 2,
    CompilationFailed = 10,
    RuntimeFailed = 11,
}

fn loop_callback(
    scope: &mut HandleScope,
    highlighter: &mut Highlighter,
) -> anyhow::Result<LoopCodes> {
    print!("{PROMPT} {}", SavePosition);
    flush();

    let mut string = String::new();
    let mut temporary_string = String::new();

    let mut cursor_index: usize = 0;

    loop {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let event::Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Enter => {
                        print!("\r");
                        break;
                    }
                    KeyCode::Up => {
                        let history_length = (*HISTORY.lock()).len();
                        if *HISTORY_POINTER.lock() == history_length {
                            continue;
                        }

                        *HISTORY_POINTER.lock() += 1;

                        if *HISTORY_POINTER.lock() == 1 {
                            temporary_string = string.clone();
                        }

                        let history = HISTORY.lock();
                        let entry = history
                            .get(history_length - *HISTORY_POINTER.lock())
                            .unwrap();
                        string = entry.to_string();
                        print!("\r{}{PROMPT} {}", Clear(ClearType::FromCursorDown), entry)
                    }
                    KeyCode::Down => {
                        let history_length = (*HISTORY.lock()).len();
                        if *HISTORY_POINTER.lock() == 0 {
                            continue;
                        }

                        *HISTORY_POINTER.lock() -= 1;

                        if *HISTORY_POINTER.lock() == 0 {
                            string = temporary_string.clone();
                        } else {
                            let history = HISTORY.lock();
                            let entry = history
                                .get(history_length - *HISTORY_POINTER.lock())
                                .unwrap();
                            string = entry.to_string();
                        };
                        print!("\r{}{PROMPT} {}", Clear(ClearType::FromCursorDown), string)
                    }
                    KeyCode::Left => {
                        // TODO: Add modifier keys to move around.
                        if cursor_index == string.len() {
                            continue;
                        }
                        cursor_index += 1;
                    }
                    KeyCode::Right => {
                        // TODO: Add modifier keys to move around.
                        if cursor_index == 0 {
                            continue;
                        }
                        cursor_index -= 1;
                    }
                    KeyCode::Char(c) => {
                        if c == 'c' && key_event.modifiers.bits() == 0b0000_0010 {
                            print!("{NEWLINE_RETURN}");
                            return Ok(LoopCodes::Cancelled);
                        }

                        string.insert(string.len() - cursor_index, c);
                    }
                    KeyCode::Backspace => {
                        // TODO: Add modifier keys for deletion.
                        // let is_alt = KeyModifiers::ALT.contains(key_event.modifiers);
                        if string.len() == 0 {
                            *HISTORY_POINTER.lock() = 0;
                            continue;
                        }
                        // string;
                        // string.clear();
                        // TODO: Fix panic on 'ä' etc.
                        string.remove(string.len() - cursor_index - 1);
                    }
                    KeyCode::Delete => {
                        // TODO: Add modifier keys for deletion.
                        if string.len() == 0 || cursor_index == 0 || cursor_index == string.len() {
                            *HISTORY_POINTER.lock() = 0;
                            continue;
                        }

                        // TODO: Fix panic on 'ä' etc.
                        string.remove(string.len() - cursor_index);
                        cursor_index -= 1
                    }
                    KeyCode::Tab => {
                        // TODO: Add detection and autocompletion for certain arguments and functions/variables.
                    }
                    _ => continue,
                }
            }
            crossterm::terminal::window_size().unwrap();
            let highlighted = highlighter.highlight(&string);
            let left = if cursor_index == 0 {
                String::new()
            } else {
                MoveLeft(cursor_index as u16).to_string()
            };
            print!(
                "{}{}{highlighted}{left}",
                RestorePosition,
                Clear(ClearType::FromCursorDown)
            );
            flush()
        }
    }

    print!("{NEWLINE_RETURN}");

    let input = string;
    *HISTORY_POINTER.lock() = 0;

    if input == "" {
        return Ok(LoopCodes::Ok);
    } else {
        (*HISTORY.lock()).push(input.clone());
    }

    let code = v8::String::new(scope, &input).unwrap();
    let script = match v8::Script::compile(scope, code, None) {
        Some(compiled_script) => compiled_script,
        None => return Ok(LoopCodes::CompilationFailed),
    };
    let result = match script.run(scope) {
        Some(result) => result,
        None => return Ok(LoopCodes::RuntimeFailed),
    };

    if !result.is_undefined() {
        print!("{}{NEWLINE_RETURN}", result.to_rust_string_lossy(scope));
    }
    return Ok(LoopCodes::Ok);
}

// fn is_valid_program(program: &str) -> bool {
//     program_exists_and_executable(program) || find_in_path(program).is_some()
// }

// fn program_exists_and_executable(program: &str) -> bool {
//     let path = Path::new(program);
//     path.exists() && path.is_executable()
// }

// fn find_in_path(program: &str) -> Option<PathBuf> {
//     env::var_os("PATH")
//         .map(|paths| {
//             env::split_paths(&paths)
//                 .filter_map(|dir| {
//                     let full_path = dir.join(program);
//                     if full_path.exists() && full_path.is_executable() {
//                         Some(full_path)
//                     } else {
//                         None
//                     }
//                 })
//                 .next()
//         })
//         .unwrap_or(None)
// }

// fn create_executable_functions(scope: &mut HandleScope, global: Local<Object>) {
//     // let executables = EXECUTABLES_TEMPLATES.lock();

//     // let code = v8::String::new(scope, &executables.join("")).unwrap();
//     // let script = v8::Script::compile(scope, code, None).unwrap();
//     // let _result = script.run(scope).unwrap();

//     // for name in EXECUTABLES.lock().iter() {
//     //     create_js_function(
//     //         scope,
//     //         global,
//     //         name,
//     //         |scope: &mut v8::HandleScope,
//     //          args: v8::FunctionCallbackArguments,
//     //          mut rv: v8::ReturnValue| {
//     //             // rv.set(args.this().into());
//     //             let get_function_name_key = v8::String::new(
//     //                 scope,
//     //                 "jssh_internal_function__get_function_name__DO_NOT_USE",
//     //             )
//     //             .unwrap()
//     //             .into();
//     //             let global = scope.get_current_context().global(scope);
//     //             let get_function_name_val = global.get(scope, get_function_name_key).unwrap();
//     //             let get_function_name =
//     //                 v8::Local::<v8::Function>::try_from(get_function_name_val).unwrap();

//     //             let null = v8::null(scope).into();

//     //             let function_name_val = get_function_name.call(scope, null, &[]).unwrap();
//     //             // let function_name = v8::Local::<v8::String>::try_from(function_name_val).unwrap();

//     //             println!("{}", function_name_val.to_rust_string_lossy(scope))

//     //             // let current_function = v8::Local::<v8::Function>::try_from(args.this()).unwrap();

//     //             // let current_function = v8::Local::<v8::Function>::try_from(args.this()).unwrap();
//     //             // let function_name = current_function.get_name(scope);

//     //             // let function_name_str = function_name.to_rust_string_lossy(scope);

//     //             // println!("{function_name_str}")
//     //         },
//     //     );
//     // }

//     // EXECUTABLES.lock().iter().for_each(|name| {
//     //     create_js_function(
//     //         scope,
//     //         global,
//     //         name,
//     //         move |scope: &mut v8::HandleScope,
//     //               args: v8::FunctionCallbackArguments,
//     //               _rv: v8::ReturnValue| {
//     //             let name = args.get(0);
//     //             if !name.is_string() {
//     //                 // TODO: Throw error
//     //                 return;
//     //             }

//     //             // scope.get_current_context().

//     //             let mut command = Command::new(name.to_rust_string_lossy(scope));
//     //             command.stdout(Stdio::piped());
//     //             command.stderr(Stdio::piped());

//     //             for i in 0..args.length() {
//     //                 let arg = args.get(i);
//     //                 // if !arg.is_string() {
//     //                 //     // TODO: Throw error
//     //                 //     return;
//     //                 // }
//     //                 command.arg(arg.to_rust_string_lossy(scope));
//     //             }
//     //             let mut child = command.spawn().unwrap();

//     //             let stdout = child.stdout.take().unwrap();
//     //             let stdout_reader = BufReader::new(stdout);

//     //             let stderr = child.stderr.take().unwrap();
//     //             let stderr_reader = BufReader::new(stderr);

//     //             let stdout_thread = std::thread::spawn(|| {
//     //                 for line in stdout_reader.lines() {
//     //                     println!("{}", line.unwrap())
//     //                 }
//     //             });

//     //             let stderr_thread = std::thread::spawn(|| {
//     //                 for line in stderr_reader.lines() {
//     //                     eprintln!("{}", line.unwrap())
//     //                 }
//     //             });

//     //             stdout_thread.join().unwrap();
//     //             stderr_thread.join().unwrap();
//     //         },
//     //     );
//     // });
// }
