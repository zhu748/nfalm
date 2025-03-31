#!/bin/bash
VERSION=$1
if [ -z "$VERSION" ]; then
    echo "Usage: ./release.sh <version> (e.g., 1.0.0)"
    exit 1
fi
git add RELEASE_NOTES.md
git commit -m "Update release notes for v$VERSION"
git push
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin "v$VERSION" --force
