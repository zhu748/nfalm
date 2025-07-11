# v0.10.4

## Bug Fixes

- Fixed reset time calculation for rate limit exceeded errors
- Improved handling of missing reset time in rate limit error responses  
- Simplified token check logic in check_token method

## Refactoring

- Renamed preprocess and context structs for better clarity in Claude middleware
- Moved system message preprocessing to middleware for better organization
