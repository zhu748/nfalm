# v0.7.16

## Improvements

- Unified API handling logic across Claude and OpenAI formats
- Centralized response transformation in a single method
- Consolidated request transformation logic
- Improved state management throughout API processing

## Bug Fixes

- Fixed object cloning issues by replacing `.clone()` with `.to_owned()`
- Resolved state management issues in async operations
