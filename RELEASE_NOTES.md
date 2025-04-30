# v0.7.18

## Improvements

- Simplified cookie handling using direct HeaderValue instead of HashMap
- Replaced manual cookie parsing with built-in cookie store for better reliability
- Optimized image upload process using stream-based async processing
- Added support for Claude's thinking mode in message parameters
- Changed system field to accept JSON values for increased flexibility
- Improved type consistency (max_tokens_to_sample from u64 to u32)
- Reduced code complexity throughout the codebase
- Streamlined HTTP request building process
- Enhanced image processing with better media type handling

## Code Quality

- Removed redundant data structures and simplified type definitions
- Improved import organization and removed unused dependencies
- Added Clone implementations to key data structures for better ergonomics
- Moved default token functions to more appropriate locations
