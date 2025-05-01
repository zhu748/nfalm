# v0.8.1

## Features

- Unified API handlers for Claude and OpenAI formats to improve consistency
- Implemented middleware-based response transformation for format-specific outputs
- Added format detection through request URI path inspection
- Created format-aware request processing pipeline

## Improvements

- Eliminated duplicate code between Claude and OpenAI handlers
- Moved stream transformation logic to utility module for better reusability
- Enhanced router configuration with middleware-based format transformations
- Improved type safety in stream transformation with generic error handling

## Code Quality

- Reduced codebase size by consolidating duplicate handlers
- Leveraged Axum's extension system to track format information through request pipeline
- Implemented more generic stream transformation with improved type parameters
- Enhanced maintainability by centralizing format-specific logic in middleware
