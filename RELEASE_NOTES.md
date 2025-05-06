# v0.9.0

## New Features
- **Google Gemini Integration**: Added support for Google Gemini API, enabling proxying of Gemini AI model requests
- **API Key Management System**: Implemented a comprehensive system for managing API keys
  - Added key rotation and validation mechanisms
  - Created dedicated KeyManager service for handling key operations
- **Enhanced Frontend Components**: 
  - New Key Management UI with visualization, submission form, and deletion capabilities
  - Updated Key API endpoints and TypeScript interfaces
- **Multi-model Support**: Clewdr now supports multiple AI models through a unified interface
  - Claude (Anthropic)
  - Gemini (Google)
  - OpenAI-compatible format for Claude

## Improvements
- **Caching System**: Implemented response caching for better performance and reduced API usage
- **Error Handling**: Enhanced error handling and retry mechanisms across all API integrations
- **Internationalization**: Added additional language support with translations for key management UI
- **State Management**: Redesigned state management with dedicated state handlers for each AI model integration

## Technical Changes
- Restructured project with separate modules for each AI model integration
- Added middleware for Gemini request preprocessing and context handling
- Implemented API format transformations between different provider formats
- Updated router with Gemini API endpoints

## Bug Fixes
- Fixed issues with streaming responses in non-streaming contexts
- Resolved authentication handling edge cases
- Improved error reporting and logging

# v0.8.3
