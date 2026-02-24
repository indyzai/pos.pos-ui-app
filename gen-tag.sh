#!/bin/bash

# Shell script to generate the next patch version git tag (semantic versioning with 'v' prefix),
# create an annotated tag, push it to remote, and store the new version locally in a file called 'latest_version.txt'.

set -euo pipefail

# File to store the latest version locally
VERSION_FILE="latest_version.txt"

PREFIX="v"
if [[ "${1:-}" == "--app" || "${1:-}" == "app" ]]; then
  PREFIX="app-v"
fi

# Fetch latest tags from remote to ensure we have up-to-date info
git fetch --tags

# Get the latest tag (sorted by version)
LATEST_TAG=$(git tag --list --sort=-version:refname | grep "^${PREFIX}[0-9]" | head -n1 || echo "")

if [ -z "$LATEST_TAG" ]; then
  # No tags exist, start with v0.0.0 and increment patch to v0.0.1 (common for first release)
  # Alternatively, use v0.0.0 if you prefer to tag the current state as initial.
  NEXT_TAG="${PREFIX}1.0.0"
  echo "No existing tags found. Using initial version: $NEXT_TAG"
else
  # Strip prefix for calculation
  LATEST_VERSION=${LATEST_TAG#$PREFIX}

  # Increment patch version using awk
  NEXT_VERSION=$(echo "$LATEST_VERSION" | awk -F. '{OFS="."; $NF+=1; print}')

  NEXT_TAG="${PREFIX}${NEXT_VERSION}"
  echo "Latest tag: $LATEST_TAG -> Next tag: $NEXT_TAG"
fi

# Create annotated tag
git tag -a "$NEXT_TAG" -m "Release $NEXT_TAG"

# Push the tag to remote (origin)
git push origin "$NEXT_TAG"

# Store the new version locally
echo "$NEXT_TAG" > "$VERSION_FILE"
echo "New version stored in $VERSION_FILE: $NEXT_TAG"

echo "Done! Tagged and pushed $NEXT_TAG"