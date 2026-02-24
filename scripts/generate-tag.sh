#!/bin/bash

# Extract version from src-tauri/tauri.conf.json
VERSION=$(grep '"version":' src-tauri/tauri.conf.json | head -n 1 | sed 's/.*"version": "\(.*\)".*/\1/')

if [ -z "$VERSION" ]; then
    echo "Error: Could not extract version from src-tauri/tauri.conf.json"
    exit 1
fi

# Check for version increment argument
INCREMENT=$1
if [[ "$INCREMENT" =~ ^(patch|minor|major)$ ]]; then
    IFS='.' read -r major minor patch <<< "$VERSION"
    
    case $INCREMENT in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
    esac
    
    NEW_VERSION="$major.$minor.$patch"
    echo "Incrementing version from $VERSION to $NEW_VERSION ($INCREMENT)..."
    
    # Update tauri.conf.json
    sed -i '' "s/\"version\": \"$VERSION\"/\"version\": \"$NEW_VERSION\"/" src-tauri/tauri.conf.json
    
    # Update package.json
    sed -i '' "s/\"version\": \"$VERSION\"/\"version\": \"$NEW_VERSION\"/" package.json
    
    # Git commit
    git add src-tauri/tauri.conf.json package.json
    git commit -m "chore: bump version to $NEW_VERSION"
    
    VERSION=$NEW_VERSION
else
    echo "No version increment specified (patch|minor|major). Using current version $VERSION."
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
    echo "Pushing changes and tag $TAG to origin..."
    git push origin main
    git push origin "$TAG"
else
    echo "To push the tag, run: git push origin $TAG"
fi
