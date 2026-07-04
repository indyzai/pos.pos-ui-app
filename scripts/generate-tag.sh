#!/bin/bash

# Extract current version from package.json (source of truth)
VERSION=$(jq -r '.version' package.json)

if [ -z "$VERSION" ] || [ "$VERSION" == "null" ]; then
    echo "Error: Could not extract version from package.json"
    exit 1
fi

# Check for version increment or specific version argument
INCREMENT=${1:-patch}
if [ "$INCREMENT" == "--push" ] || [ "$INCREMENT" == "--amend" ]; then
    INCREMENT="patch"
fi
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
    echo "Bumping version to $VERSION in package.json and tauri.conf.json..."
    
    # Update package.json
    jq ".version = \"$VERSION\"" package.json > package.json.tmp && mv package.json.tmp package.json
    
    # Update tauri.conf.json
    jq ".version = \"$VERSION\"" src-tauri/tauri.conf.json > tauri.conf.json.tmp && mv tauri.conf.json.tmp src-tauri/tauri.conf.json
    
    # Commit changes
    git add package.json src-tauri/tauri.conf.json
    
    # Check for --amend flag
    if [[ "$*" == *"--amend"* ]]; then
        LAST_MSG=$(git log -1 --pretty=%B)
        git commit --amend -m "${LAST_MSG} - bumped to v${VERSION}"
    else
        git commit -m "chore: bump version to $VERSION"
    fi
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
    if [ -n "$NEW_VERSION" ]; then
        echo "Pushing commit and tag $TAG to origin..."
        git push origin
        git push origin "$TAG"
    else
        echo "Pushing tag $TAG to origin..."
        git push origin "$TAG"
    fi
else
    echo "To push the tag, run: git push origin $TAG"
    if [ -n "$NEW_VERSION" ]; then
        echo "And don't forget to push the version bump commit: git push origin"
    fi
fi
