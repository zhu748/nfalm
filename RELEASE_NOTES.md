# v0.10.1

## 🚀 New Features

- **Memory Performance**: Added mimalloc as the global allocator for improved memory management and performance

## 🐛 Bug Fixes

- **Claude Processing**: Enhanced stop sequence handling in ClaudePreprocess to use `take()` for better ownership management
- **Model Handling**: Improved model handling in ClaudePreprocess and enhanced cookie collection logic
- **Dependencies**: Updated toml dependency from version 0.9.0 to 0.9.1

## 📚 Documentation

- Added acknowledgements section to README files (both English and Chinese versions)

## 🔧 Technical Improvements

- Better memory ownership patterns in request processing
- Enhanced cookie management reliability
- Improved error handling in Claude preprocessing pipeline
