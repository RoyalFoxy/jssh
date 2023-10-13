# JavaScript Shell

Imagine a world where you can use your favorite scripting language (JavaScript) in your terminal as a replacement for bash, zsh or fish.

- [JavaScript Shell](#javascript-shell)
  - [What is JSSh](#what-is-jssh)
  - [Incomplete feature list](#incomplete-feature-list)
  - [An incomplete "bug" list](#an-incomplete-bug-list)
  - [A list of features that will likely get implemented](#a-list-of-features-that-will-likely-get-implemented)
  - [A list of things I want to look into](#a-list-of-things-i-want-to-look-into)
  - [Regarding this code base](#regarding-this-code-base)
  - [How to build](#how-to-build)
    - [Cloning](#cloning)
    - [Dependencies](#dependencies)
    - [Building](#building)
  - [Installation](#installation)

## What is JSSh

JSSh or JavaScript Shell is a somewhat early approach to a REPL for JavaScript with the power to execute any executable in the path and more. It currently is in a very early state of development.

## Incomplete feature list

- Builtin functions like `$exit()`, `$source()` and `$run()`.
- full javascript support thanks to rusty_v8.
- all executables in the path are mapped to functions in javascript (e.g. `ls()`, `pwd()`).
- You have all environment variables as normal variables.
- Ability to run system commands with `$run`.
- Custom implementation of `cd` as the `cd` executable cannot mutate the cwd of another process.

## An incomplete "bug" list

- Nothing throws errors if wrong arguments are passed
- Almost everything uses `unwrap()` (a rust thing) which panics the program if a there's a None value or an error occurs.
- You cannot delete any characters that are "stitched together" (ä, é etc.).
- Sourcing a file twice which declares a variable or a function crashes the shell as variable shadowing only exists for child scopes in javascript and cannot be done in the same scope. This can be easily fixed by addressing the `unwrap()` issue.

## A list of features that will likely get implemented

- Error handling lol
- multiline support for input.
- Moving around input with option, command & control
custom prompt support.
- Autocompletion of variables, functions and path arguments (like node.js repl).
- `console.log` & `console.error` (currently only echo works).
- Expand on functionality and advantages over other shells.

## A list of things I want to look into

- Currently every executable just dumps their output to stdout/stderr but then you cannot use javascript to do stuff with it. A solution may be streams but thats a big may. A problem with this approach is that every executable would have to return an object which has a type field which either is `"out"` or `"err"` and a content field with the actual content so to not loose the meaning of text that's printed out. This however would introduce tons and tons of objects which could increase memory consumption/performance used by the garbage collector drastically...
- Better syntax highlighting.
- Other stuff I probably forgot when typing this.

## Regarding this code base

This is and likely will stay a hobby project of mine so I cannot push out features anytime of the day.

The code written should also not be taken as the gold standard as this is my first time writing a bigger project in rust, first time using the v8 engine and the first time building a shell...

## How to build

### Cloning

In order to build this project you would have to clone the repository with `git clone https://github.com/RoyalFoxy/jssh.git` or `git clone git@github.com:RoyalFoxy/jssh.git`.

### Dependencies

The dependencies are quite minimal

First you obviously need rust installed on your system. visit [rust-lang](https://www.rust-lang.org/learn/get-started) to get started.

As `rusty-v8` contains the v8 engine as a binary you do not have to build it yourself but only the wrapper that is `rusty-v8`. If you want to build v8 yourself go over to the [binary-build](https://github.com/denoland/rusty_v8#binary-build) section on their github repo.

### Building

As simple as running `cargo build`.

## Installation

Currently there is no way of installing JSSh as it is too early imo to be used as a primary shell. If you still want to use it look at how you can build it [here](#how-to-build)
