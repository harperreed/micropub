# Homebrew Release Workflow Design

**Date:** 2025-11-30
**Status:** Validated
**Purpose:** Automate Homebrew tap updates with pre-compiled bottles for macOS

## Overview

Design a GitHub Actions workflow that automatically updates the Homebrew tap (`harperreed/homebrew-tap`) when new releases are created, including building bottles (pre-compiled binaries) for macOS platforms.

## Release Flow Architecture

```
Tag Push (v0.2.0)
    ↓
Release Workflow (.github/workflows/release.yml)
    ├─ Build release binaries (macOS Intel, macOS ARM, Linux)
    ├─ Run tests
    ├─ Create GitHub Release
    ├─ Attach binaries as release assets
    └─ Publish to crates.io
    ↓
Homebrew Workflow (.github/workflows/homebrew.yml) [NEW]
    ├─ Build bottles for macOS (Intel + ARM)
    ├─ Calculate SHA256 for source tarball
    ├─ Calculate SHA256 for bottles
    ├─ Generate/update Formula/micropub.rb
    └─ Auto-commit to harperreed/homebrew-tap
```

## Design Decisions

### 1. Trigger Mechanism
**Choice:** Release creation (`release: [published]` event)

**Reasoning:**
- Tag push creates the release (existing workflow)
- Release creation triggers Homebrew update (new workflow)
- Clean separation of concerns
- Allows manual verification between steps if needed

### 2. Binary Distribution Strategy
**Choice:** Hybrid approach - bottles for macOS, source for Linux

**Why:**
- macOS users prefer bottles (fast install, no Rust needed)
- Linux users typically have build tools
- Standard practice for Rust CLI tools
- Reduces workflow complexity vs building for all platforms

### 3. Formula Update Method
**Choice:** Direct auto-commit to tap repository

**Why:**
- Formula updates are mechanical (version + SHA256)
- Release already verified before workflow triggers
- Faster availability for users
- Can revert if issues found
- No manual intervention needed

### 4. Target Platforms for Bottles
**Choice:**
- `x86_64-apple-darwin` (Intel Mac)
- `aarch64-apple-darwin` (Apple Silicon)

**Why:**
- Covers all modern macOS users
- GitHub Actions provides both runner types
- Linux users can build from source

## Component Details

### Modified: `.github/workflows/release.yml`

**Current behavior:**
- Detects version changes in Cargo.toml
- Builds and tests
- Creates tag and GitHub Release
- Publishes to crates.io

**Required changes:**
1. Add cross-compilation for multiple targets
2. Package binaries as `.tar.gz` artifacts
3. Upload artifacts to GitHub Release
4. Handle case where tag already exists (idempotent)

**Artifacts to create:**
- `micropub-v{VERSION}-x86_64-apple-darwin.tar.gz`
- `micropub-v{VERSION}-aarch64-apple-darwin.tar.gz`
- `micropub-v{VERSION}-x86_64-unknown-linux-gnu.tar.gz`

### New: `.github/workflows/homebrew.yml`

**Trigger:**
```yaml
on:
  release:
    types: [published]
```

**Job 1: Build Bottles**
- **Runner:** macOS (both Intel and ARM)
- **Steps:**
  1. Checkout code at release tag
  2. Install Rust toolchain
  3. Build with `cargo build --release`
  4. Create bottle tarball
  5. Calculate SHA256
  6. Upload as workflow artifact

**Job 2: Update Formula**
- **Runner:** Ubuntu (faster, cheaper)
- **Steps:**
  1. Download source tarball from GitHub Release
  2. Calculate source tarball SHA256
  3. Download bottle artifacts from Job 1
  4. Extract bottle SHA256s
  5. Generate `Formula/micropub.rb` with template
  6. Clone `harperreed/homebrew-tap`
  7. Update/create formula file
  8. Commit and push changes

**Authentication:**
- Requires `HOMEBREW_TAP_TOKEN` secret
- Personal Access Token with `repo` scope
- Write access to `harperreed/homebrew-tap`

### Homebrew Formula Structure

**File:** `Formula/micropub.rb` in `harperreed/homebrew-tap`

```ruby
class Micropub < Formula
  desc "Ultra-compliant Micropub CLI with MCP server support"
  homepage "https://github.com/harperreed/micropub"
  url "https://github.com/harperreed/micropub/archive/refs/tags/v{VERSION}.tar.gz"
  sha256 "{SOURCE_SHA256}"
  license "MIT"

  depends_on "rust" => :build

  bottle do
    root_url "https://github.com/harperreed/micropub/releases/download/v{VERSION}"
    sha256 cellar: :any_skip_relocation, arm64_sonoma: "{ARM64_SHA256}"
    sha256 cellar: :any_skip_relocation, ventura: "{X86_64_SHA256}"
  end

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/micropub --version")
  end
end
```

**Dynamic values (calculated by workflow):**
- `{VERSION}` - Git tag version (e.g., 0.2.0)
- `{SOURCE_SHA256}` - SHA256 of source tarball
- `{ARM64_SHA256}` - SHA256 of ARM64 bottle
- `{X86_64_SHA256}` - SHA256 of x86_64 bottle

## User Experience

### Installation
```bash
# Add tap (one time)
brew tap harperreed/tap

# Install micropub
brew install micropub
```

### Updates
```bash
# Update tap formulas
brew update

# Upgrade micropub
brew upgrade micropub
```

### Behind the scenes
1. User pushes tag `v0.2.1`
2. Release workflow creates GitHub Release (~2-3 minutes)
3. Homebrew workflow updates tap (~5-7 minutes)
4. Users can `brew upgrade micropub` immediately

## Security Considerations

### Token Permissions
- `HOMEBREW_TAP_TOKEN` needs minimal scope: `repo` access to `homebrew-tap` only
- Use fine-grained tokens if possible
- Rotate periodically

### Bottle Integrity
- Bottles built on GitHub-hosted runners (trusted environment)
- SHA256 verification ensures integrity
- Users can verify with `brew --verbose install micropub`

### Formula Safety
- Auto-commits reviewed via git history
- Formula syntax validated by Homebrew on user install
- Revert capability via git

## Testing Strategy

### Before Merge
1. Test release workflow with manual tag
2. Verify artifacts uploaded correctly
3. Test Homebrew workflow with test release
4. Validate formula syntax with `brew audit`

### After Deployment
1. Install from tap: `brew install harperreed/tap/micropub`
2. Verify version: `micropub --version`
3. Test basic functionality
4. Create new release and verify auto-update

## Rollback Plan

### If formula breaks
```bash
# Revert commit in homebrew-tap
git revert HEAD
git push

# Users uninstall/reinstall
brew uninstall micropub
brew install micropub
```

### If workflow fails
- Workflow failure doesn't affect existing installations
- Fix workflow and re-run on same release
- Or manually update formula

## Future Enhancements

### Potential additions (not in initial scope)
- Linux bottles (x86_64-unknown-linux-gnu)
- Windows support via Chocolatey/Scoop
- Homebrew core submission (after stability proven)
- Automated testing in formula (integration tests)
- Multi-version support (install specific versions)

### Metrics to track
- Installation count (GitHub Release downloads)
- Formula update success rate
- Build time for bottles
- User-reported issues

## Implementation Order

1. Create `HOMEBREW_TAP_TOKEN` and add to repository secrets
2. Modify `.github/workflows/release.yml` for cross-compilation
3. Create `.github/workflows/homebrew.yml` with bottle building
4. Test with v0.2.0 release (already created)
5. Verify formula in tap repository
6. Document installation in README.md

## Success Criteria

- [x] Design validated with stakeholder
- [ ] Workflows implemented and tested
- [ ] Formula successfully updates on release
- [ ] Users can install via `brew install harperreed/tap/micropub`
- [ ] Bottles work on both Intel and ARM Macs
- [ ] Documentation updated with installation instructions

## References

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Creating Bottles](https://docs.brew.sh/Bottles)
- [GitHub Actions - Release Events](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#release)
- [Rust Cross-Compilation](https://rust-lang.github.io/rustup/cross-compilation.html)
