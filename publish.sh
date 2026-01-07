#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🐕 Fido Publish Script${NC}"
echo "========================"

# Extract versions
WORKSPACE_VERSION=$(grep -A1 '^\[workspace\.package\]' Cargo.toml | grep 'version' | sed 's/.*"\(.*\)".*/\1/')
TYPES_DEP_VERSION=$(grep 'fido-types.*version' Cargo.toml | grep -v workspace | sed 's/.*version = "\(.*\)".*/\1/')

echo -e "\nWorkspace version: ${GREEN}$WORKSPACE_VERSION${NC}"
echo -e "fido-types dependency version: ${GREEN}$TYPES_DEP_VERSION${NC}"

# Check version mismatch
if [ "$WORKSPACE_VERSION" != "$TYPES_DEP_VERSION" ]; then
    echo -e "\n${RED}❌ Version mismatch!${NC}"
    echo "Workspace version ($WORKSPACE_VERSION) doesn't match fido-types dependency ($TYPES_DEP_VERSION)"
    echo "Update the fido-types version in root Cargo.toml before publishing."
    exit 1
fi

# Check if versions are already published
echo -e "\n${YELLOW}Checking crates.io for existing versions...${NC}"

TYPES_PUBLISHED=$(cargo search fido-types 2>/dev/null | grep "^fido-types" | sed 's/.*= "\(.*\)".*/\1/' || echo "")
TUI_PUBLISHED=$(cargo search fido 2>/dev/null | grep "^fido = " | sed 's/.*= "\(.*\)".*/\1/' || echo "")

echo "fido-types on crates.io: ${TYPES_PUBLISHED:-not found}"
echo "fido on crates.io: ${TUI_PUBLISHED:-not found}"

if [ "$TYPES_PUBLISHED" = "$WORKSPACE_VERSION" ]; then
    echo -e "${YELLOW}⚠️  fido-types $WORKSPACE_VERSION already published${NC}"
    SKIP_TYPES=true
else
    SKIP_TYPES=false
fi

if [ "$TUI_PUBLISHED" = "$WORKSPACE_VERSION" ]; then
    echo -e "${YELLOW}⚠️  fido $WORKSPACE_VERSION already published${NC}"
    SKIP_TUI=true
else
    SKIP_TUI=false
fi

if [ "$SKIP_TYPES" = true ] && [ "$SKIP_TUI" = true ]; then
    echo -e "\n${GREEN}✓ Both crates already at version $WORKSPACE_VERSION. Nothing to publish.${NC}"
    exit 0
fi

# Run dry-run for fido-types first
if [ "$SKIP_TYPES" = false ]; then
    echo -e "\n${YELLOW}Running dry-run for fido-types...${NC}"
    if ! cargo publish --dry-run -p fido-types 2>&1; then
        echo -e "${RED}❌ fido-types dry-run failed${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ fido-types dry-run passed${NC}"
fi

# Note: fido dry-run will fail if fido-types isn't published yet (dependency not on crates.io)
# We'll do the fido dry-run after publishing fido-types if needed
if [ "$SKIP_TYPES" = true ] && [ "$SKIP_TUI" = false ]; then
    echo -e "\n${YELLOW}Running dry-run for fido...${NC}"
    if ! cargo publish --dry-run -p fido 2>&1; then
        echo -e "${RED}❌ fido dry-run failed${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓ fido dry-run passed${NC}"
elif [ "$SKIP_TYPES" = false ] && [ "$SKIP_TUI" = false ]; then
    echo -e "${YELLOW}Note: fido dry-run will run after fido-types is published (dependency chain)${NC}"
fi

# Confirm with user
echo -e "\n${YELLOW}Ready to publish:${NC}"
[ "$SKIP_TYPES" = false ] && echo "  - fido-types $WORKSPACE_VERSION"
[ "$SKIP_TUI" = false ] && echo "  - fido $WORKSPACE_VERSION"

read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Publish fido-types first (if needed)
if [ "$SKIP_TYPES" = false ]; then
    echo -e "\n${YELLOW}Publishing fido-types...${NC}"
    cargo publish -p fido-types
    echo -e "${GREEN}✓ fido-types published${NC}"
    
    # Wait for crates.io to index
    echo "Waiting 20s for crates.io to index..."
    sleep 20
    
    # Now do dry-run for fido (dependency should be available)
    if [ "$SKIP_TUI" = false ]; then
        echo -e "\n${YELLOW}Running dry-run for fido...${NC}"
        if ! cargo publish --dry-run -p fido 2>&1; then
            echo -e "${RED}❌ fido dry-run failed${NC}"
            echo "fido-types was published but fido failed. You may need to publish fido manually."
            exit 1
        fi
        echo -e "${GREEN}✓ fido dry-run passed${NC}"
    fi
fi

# Publish fido TUI
if [ "$SKIP_TUI" = false ]; then
    echo -e "\n${YELLOW}Publishing fido...${NC}"
    cargo publish -p fido
    echo -e "${GREEN}✓ fido published${NC}"
fi

echo -e "\n${GREEN}🎉 Done! Published version $WORKSPACE_VERSION${NC}"
