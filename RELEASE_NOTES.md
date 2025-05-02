# v0.8.2

## Highlights

- Refactored middleware architecture for improved API format compatibility
- Enhanced cookie validation and management
- Improved configuration options
- Better error handling and response processing
- Added web search functionality

## Changes

### New Features

- **Web Search**: Added web search capability for enhanced contextual responses

### Middleware Improvements

- **New Request Processor**: Added request preprocessing middleware to unify different API formats (Claude and OpenAI)
- **Response Transformation**: Enhanced streaming response handling to ensure compatibility between Claude and OpenAI formats
- **Test Message Detection**: Added automatic detection and handling of test messages from client applications

### Cookie Management

- **Improved Validation**: Enhanced cookie format validation and error reporting
- **Better Display**: Added ellipsis cookie display for improved UI readability
- **Reset Handling**: Refined cookie reset time management

### Configuration

- **Organized Settings**: Better structured configuration options with improved documentation
- **Cache Settings**: Enhanced cache configuration with system message and last N message options
- **Prompt Settings**: Refined prompt configuration options

### Frontend

- **Updated Translations**: Improved English and Chinese localization
- **Config UI**: Enhanced configuration form with better organization and tooltips
- **Status Display**: Improved cookie status display

### Other Improvements

- **Error Handling**: More robust error handling throughout the application
- **Performance**: Optimized response processing and streaming
- **Package Management**: Added pnpm workspace configuration
