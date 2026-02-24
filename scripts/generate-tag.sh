#!/bin/bash

# Extract version from src-tauri/tauri.conf.json
VERSION=$(grep '"version":' src-tauri/tauri.conf.json | head -n 1 | sed 's/.*"version": "\(.*\)".*/\1/')

if [ -z "$VERSION" ]; then
    echo "Error: Could not extract version from src-tauri/tauri.conf.json"
    exit 1
fi

TAG="v$VERSION"

# Check for --push flag
PUSH=false
if [[ "$*" == *"--push"* ]]; then
    PUSH=true
fi

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Tag $TAG already exists."
else
    echo "Creating tag $TAG..."
    git tag "$TAG"
    echo "Tag $TAG created successfully."
fi

if [ "$PUSH" = true ]; then
    echo "Pushing tag $TAG to origin..."
    git push origin "$TAG"
else
    echo "To push the tag, run: git push origin $TAG"
fi
