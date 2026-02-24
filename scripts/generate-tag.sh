#!/bin/bash

# Extract current version from package.json (source of truth)
VERSION=$(jq -r '.version' package.json)

if [ -z "$VERSION" ] || [ "$VERSION" == "null" ]; then
    echo "Error: Could not extract version from package.json"
    exit 1
fi

# Check for version increment or specific version argument
INCREMENT=$1
NEW_VERSION=""

if [[ "$INCREMENT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    NEW_VERSION=$INCREMENT
elif [[ "$INCREMENT" =~ ^(patch|minor|major)$ ]]; then
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
fi

if [ -n "$NEW_VERSION" ]; then
    VERSION=$NEW_VERSION
else
    echo "Using current version $VERSION (no valid increment/version provided)."
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
    # Note: We only push the tag, not the branch, as requested (no bump commit)
    git push origin "$TAG"
else
    echo "To push the tag, run: git push origin $TAG"
fi
