#!/usr/bin/env bash
set -euo pipefail

# -----------------------------------------------------------------------------
# Release Script for Dummy-Bot
# Automates:
#   1. Quality verification gate (fmt, clippy, test)
#   2. Version bumping in Cargo.toml & Cargo.lock
#   3. Merge develop into main (--no-ff)
#   4. Tag creation (vX.Y.Z)
#   5. Push branches and release tag to origin (triggers GitHub Actions)
#   6. Checkout back to develop
# -----------------------------------------------------------------------------
# Nâng bản vá (patch: 3.4.0 -> 3.4.1)
# ./scripts/release.sh patch "Release v3.4.1: Bug fixes"
#
# Nâng bản tính năng (minor: 3.4.0 -> 3.5.0)
# ./scripts/release.sh minor "Release v3.5.0: New feature set"
#
# Nâng bản lớn (major: 3.4.0 -> 4.0.0)
# ./scripts/release.sh major "Release v4.0.0: Major rewrite"
#
# Chỉ định phiên bản cụ thể
# ./scripts/release.sh 3.4.1 "Release v3.4.1: Custom notes"
#
# Chạy thử nghiệm mà không ghi đè commit/push (Dry Run)
# ./scripts/release.sh patch --dry-run

COLOR_RED="\033[0;31m"
COLOR_GREEN="\033[0;32m"
COLOR_YELLOW="\033[0;33m"
COLOR_BLUE="\033[0;34m"
COLOR_RESET="\033[0m"

info() {
    echo -e "${COLOR_BLUE}[INFO]${COLOR_RESET} $*"
}

success() {
    echo -e "${COLOR_GREEN}[SUCCESS]${COLOR_RESET} $*"
}

warn() {
    echo -e "${COLOR_YELLOW}[WARN]${COLOR_RESET} $*"
}

error() {
    echo -e "${COLOR_RED}[ERROR]${COLOR_RESET} $*" >&2
}

abort() {
    error "$*"
    exit 1
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

DRY_RUN=false
VERSION_INPUT=""
RELEASE_MESSAGE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [patch|minor|major|<version>] [\"Release message\"] [--dry-run]"
            echo ""
            echo "Examples:"
            echo "  $0 patch"
            echo "  $0 minor \"Release v3.5.0: New feature set\""
            echo "  $0 3.4.1 \"Release v3.4.1: Bug fixes\""
            echo "  $0 patch --dry-run"
            exit 0
            ;;
        *)
            if [[ -z "${VERSION_INPUT}" ]]; then
                VERSION_INPUT="$1"
            elif [[ -z "${RELEASE_MESSAGE}" ]]; then
                RELEASE_MESSAGE="$1"
            fi
            shift
            ;;
    esac
done

# 1. Pre-condition checks
info "Checking working tree and Git repository status..."

if [[ ! -f "Cargo.toml" ]]; then
    abort "Cargo.toml not found in ${ROOT_DIR}"
fi

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "${CURRENT_BRANCH}" != "develop" ]]; then
    abort "You must run this script from the 'develop' branch. Current branch: ${CURRENT_BRANCH}"
fi

if [[ -n "$(git status --porcelain)" ]]; then
    abort "Working tree is dirty. Commit or stash your changes before releasing."
fi

info "Fetching latest references from origin..."
git fetch origin main develop --tags

# Ensure local develop is up to date with origin/develop
LOCAL_DEVELOP_REV="$(git rev-parse develop)"
REMOTE_DEVELOP_REV="$(git rev-parse origin/develop 2>/dev/null || echo "")"
if [[ -n "${REMOTE_DEVELOP_REV}" && "${LOCAL_DEVELOP_REV}" != "${REMOTE_DEVELOP_REV}" ]]; then
    # Check if local is behind remote
    if ! git merge-base --is-ancestor origin/develop develop; then
        abort "Local 'develop' is behind 'origin/develop'. Please run 'git pull origin develop' first."
    fi
fi

# 2. Extract and compute version
CURRENT_VERSION="$(grep -m1 '^version\s*=' Cargo.toml | sed -E 's/version\s*=\s*"([^"]+)".*/\1/')"
info "Current version: ${CURRENT_VERSION}"

IFS='.' read -r MAJOR MINOR PATCH <<< "${CURRENT_VERSION}"

if [[ -z "${VERSION_INPUT}" ]]; then
    echo -e "${COLOR_YELLOW}Select release bump type:${COLOR_RESET}"
    echo "  1) patch ($MAJOR.$MINOR.$((PATCH + 1)))"
    echo "  2) minor ($MAJOR.$((MINOR + 1)).0)"
    echo "  3) major ($((MAJOR + 1)).0.0)"
    echo "  4) custom"
    read -rp "Enter choice [1-4]: " CHOICE
    case "${CHOICE}" in
        1) VERSION_INPUT="patch" ;;
        2) VERSION_INPUT="minor" ;;
        3) VERSION_INPUT="major" ;;
        4)
            read -rp "Enter target version (e.g. 3.4.1): " VERSION_INPUT
            ;;
        *)
            abort "Invalid choice"
            ;;
    esac
fi

case "${VERSION_INPUT}" in
    patch)
        TARGET_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))"
        ;;
    minor)
        TARGET_VERSION="${MAJOR}.$((MINOR + 1)).0"
        ;;
    major)
        TARGET_VERSION="$((MAJOR + 1)).0.0"
        ;;
    v*|V*)
        TARGET_VERSION="${VERSION_INPUT#[vV]}"
        ;;
    *)
        TARGET_VERSION="${VERSION_INPUT}"
        ;;
esac

# Validate target version format (SemVer)
if [[ ! "${TARGET_VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    abort "Invalid target version format: '${TARGET_VERSION}'. Must follow SemVer (e.g. 3.4.1)."
fi

TARGET_TAG="v${TARGET_VERSION}"

# Check if tag already exists
if git rev-parse "${TARGET_TAG}" >/dev/null 2>&1 || git ls-remote --tags origin "refs/tags/${TARGET_TAG}" | grep -q "${TARGET_TAG}"; then
    abort "Tag ${TARGET_TAG} already exists locally or on remote origin!"
fi

if [[ -z "${RELEASE_MESSAGE}" ]]; then
    RELEASE_MESSAGE="Release ${TARGET_TAG}"
fi

info "Target version: ${TARGET_VERSION} (Tag: ${TARGET_TAG})"
info "Release message: ${RELEASE_MESSAGE}"

if [[ "${DRY_RUN}" == "true" ]]; then
    warn "DRY-RUN mode enabled. No changes, commits, or pushes will be executed."
fi

# 3. Quality Verification Gate
info "Running quality verification gate..."
info "1/3 Checking code formatting (cargo fmt)..."
cargo fmt --check

info "2/3 Checking linter warnings (cargo clippy)..."
cargo clippy --locked --all-targets -- -D warnings

info "3/3 Running test suite (cargo test)..."
cargo test --locked

success "All quality verification checks passed!"

if [[ "${DRY_RUN}" == "true" ]]; then
    success "Dry run completed successfully. Everything looks ready for release ${TARGET_TAG}."
    exit 0
fi

# 4. Bump version in Cargo.toml & Cargo.lock
info "Bumping version in Cargo.toml to ${TARGET_VERSION}..."
sed -i -E "s/^version\s*=\s*\"[^\"]+\"/version = \"${TARGET_VERSION}\"/" Cargo.toml

info "Updating Cargo.lock..."
cargo check --quiet

git add Cargo.toml Cargo.lock
git commit -m "chore(release): bump version to ${TARGET_VERSION}"
success "Committed version bump to ${TARGET_VERSION} on develop."

# 5. Merge develop into main
info "Checking out 'main' branch..."
git checkout main

info "Pulling latest changes on 'main' from origin..."
git pull origin main

info "Merging 'develop' into 'main' with --no-ff..."
git merge --no-ff develop -m "chore(release): merge develop into main for ${TARGET_TAG}"
success "Merged develop into main."

# 6. Create annotated release tag
info "Creating annotated tag ${TARGET_TAG}..."
git tag -a "${TARGET_TAG}" -m "${RELEASE_MESSAGE}"
success "Created tag ${TARGET_TAG}."

# 7. Push to origin
info "Pushing 'main', 'develop', and tag '${TARGET_TAG}' to origin..."
git push origin main develop
git push origin "${TARGET_TAG}"
success "Pushed main, develop, and ${TARGET_TAG} to origin."

# 8. Return to develop
info "Checking out 'develop' branch..."
git checkout develop

success "=== Release ${TARGET_TAG} completed successfully! ==="
info "GitHub Actions workflow (.github/workflows/release.yml) has been triggered."
info "You can monitor the build and release publish at: https://github.com/BlaskVN/Dummy-Bot/actions"
