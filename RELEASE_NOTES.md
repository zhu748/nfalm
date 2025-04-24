# v0.7.6

## Feature

- Add Dockerfile for building the image
- Support DeepSeek reasoning format for OpenAI compatible mode

## Improvement

- Hand written stream transformer instead of using `transform_stream` library,
  massively improved performance and simplified code
