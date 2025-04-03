# v0.4.0

## Feature

- Support concurrent!

## Improvement

- Refactor architecture to support concurrent execution of multiple tasks.
- Use channel instead of mutex, so deadlock will not happen.
