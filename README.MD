# Clewd<span style="color:#CE422B">R</span>

Total rewrite of [Clewd修改版](https://github.com/teralomaniac/clewd) in Rust.

## **WORK IN PROGRESS**

## Advantages

1. **No Node.js**
~~Although I'm sure all of you playing SillyTavern have it installed~~
2. **No external binaries**
Clewd spawns a `superfetch` command for every request to mimic browser behavior. ClewdR uses native Rust `rquest` library.
3. **Support macOS**
Clewd doesn't provide `superfetch` for macOS, so it doesn't work on macOS.
